use std::collections::BTreeMap;
use std::num::NonZeroU64;

use mawr_core::{ElementRef, OperationFailure, Property, ResourceKind, SemanticRole, SessionId};
use mawr_semantic_html::ExtractedSemanticUnit;

use crate::model::{
    ReferenceAssignment, ReferenceAssignmentReason, ReferenceLoss, ReferenceLossReason, StoredState,
};

#[derive(Debug)]
pub(crate) struct MatchResult {
    pub(crate) references: Vec<ElementRef>,
    pub(crate) assignments: Vec<ReferenceAssignment>,
    pub(crate) losses: Vec<ReferenceLoss>,
    pub(crate) preserved: usize,
    pub(crate) next_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticKey {
    role: SemanticRole,
    name: Option<String>,
    destination: Option<String>,
}

pub(crate) fn assign(
    session: SessionId,
    next_sequence: u64,
    previous: Option<&StoredState>,
    current: &[ExtractedSemanticUnit],
    reset_reason: Option<mawr_core::ResetReason>,
) -> Result<MatchResult, OperationFailure> {
    let Some(previous) = previous.filter(|_| reset_reason.is_none()) else {
        return allocate_all(
            session,
            next_sequence,
            current,
            reset_reason.map_or(
                ReferenceAssignmentReason::InitialState,
                ReferenceAssignmentReason::Reset,
            ),
            previous,
            reset_reason,
        );
    };

    let old = previous.units();
    let mut current_refs = vec![None; current.len()];
    let mut current_reasons = vec![None; current.len()];
    let mut old_used = vec![false; old.len()];
    let mut old_loss_reasons = vec![None; old.len()];

    match_page(
        old,
        current,
        &mut old_used,
        &mut current_refs,
        &mut current_reasons,
    );
    match_author_ids(
        old,
        current,
        &mut old_used,
        &mut old_loss_reasons,
        &mut current_refs,
        &mut current_reasons,
    );
    match_semantic_keys(
        old,
        current,
        &mut old_used,
        &mut old_loss_reasons,
        &mut current_refs,
        &mut current_reasons,
    );

    let mut sequence = next_sequence;
    let mut references = Vec::with_capacity(current.len());
    let mut assignments = Vec::with_capacity(current.len());
    let mut preserved = 0;
    for (index, unit) in current.iter().enumerate() {
        let (reference, reason) = if let Some(reference) = current_refs[index] {
            preserved += 1;
            (
                reference,
                current_reasons[index].expect("preserved reference has a reason"),
            )
        } else {
            let reference = allocate_reference(session, &mut sequence)?;
            let reason = current_reasons[index].unwrap_or_else(|| assignment_reason(unit));
            (reference, reason)
        };
        references.push(reference);
        assignments.push(ReferenceAssignment::new(unit.source(), reference, reason));
    }

    let losses = old
        .iter()
        .enumerate()
        .filter(|(index, _)| !old_used[*index])
        .map(|(index, unit)| {
            ReferenceLoss::new(
                unit.reference(),
                old_loss_reasons[index].unwrap_or_else(|| unmatched_loss_reason(unit.semantic())),
            )
        })
        .collect();

    Ok(MatchResult {
        references,
        assignments,
        losses,
        preserved,
        next_sequence: sequence,
    })
}

fn allocate_all(
    session: SessionId,
    next_sequence: u64,
    current: &[ExtractedSemanticUnit],
    reason: ReferenceAssignmentReason,
    previous: Option<&StoredState>,
    reset_reason: Option<mawr_core::ResetReason>,
) -> Result<MatchResult, OperationFailure> {
    let mut sequence = next_sequence;
    let mut references = Vec::with_capacity(current.len());
    let mut assignments = Vec::with_capacity(current.len());
    for unit in current {
        let reference = allocate_reference(session, &mut sequence)?;
        references.push(reference);
        assignments.push(ReferenceAssignment::new(unit.source(), reference, reason));
    }
    let losses = previous
        .into_iter()
        .flat_map(StoredState::units)
        .map(|unit| {
            ReferenceLoss::new(
                unit.reference(),
                ReferenceLossReason::Reset(
                    reset_reason.expect("previous state reset has a reason"),
                ),
            )
        })
        .collect();
    Ok(MatchResult {
        references,
        assignments,
        losses,
        preserved: 0,
        next_sequence: sequence,
    })
}

fn match_page(
    old: &[crate::StoredSemanticUnit],
    current: &[ExtractedSemanticUnit],
    old_used: &mut [bool],
    current_refs: &mut [Option<ElementRef>],
    current_reasons: &mut [Option<ReferenceAssignmentReason>],
) {
    let old_pages = old
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.semantic().role() == SemanticRole::Page)
        .collect::<Vec<_>>();
    let current_pages = current
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.role() == SemanticRole::Page)
        .collect::<Vec<_>>();
    if let ([(old_index, old_page)], [(current_index, _)]) =
        (old_pages.as_slice(), current_pages.as_slice())
    {
        old_used[*old_index] = true;
        current_refs[*current_index] = Some(old_page.reference());
        current_reasons[*current_index] = Some(ReferenceAssignmentReason::UniqueSemanticIdentity);
    }
}

fn match_author_ids(
    old: &[crate::StoredSemanticUnit],
    current: &[ExtractedSemanticUnit],
    old_used: &mut [bool],
    old_loss_reasons: &mut [Option<ReferenceLossReason>],
    current_refs: &mut [Option<ElementRef>],
    current_reasons: &mut [Option<ReferenceAssignmentReason>],
) {
    let old_keys = buckets(
        old.iter()
            .enumerate()
            .filter_map(|(index, unit)| author_id(unit.semantic()).map(|key| (key, index))),
    );
    let current_keys = buckets(
        current
            .iter()
            .enumerate()
            .filter_map(|(index, unit)| author_id(unit).map(|key| (key, index))),
    );

    for (key, current_indices) in &current_keys {
        let old_indices = old_keys.get(key).map_or(&[][..], Vec::as_slice);
        if let ([current_index], [old_index]) = (current_indices.as_slice(), old_indices)
            && old[*old_index].semantic().role() == current[*current_index].role()
        {
            old_used[*old_index] = true;
            current_refs[*current_index] = Some(old[*old_index].reference());
            current_reasons[*current_index] = Some(ReferenceAssignmentReason::UniqueAuthorId);
        } else if current_indices.len() > 1 || old_indices.len() > 1 {
            for index in current_indices {
                current_reasons[*index] = Some(ReferenceAssignmentReason::AmbiguousAuthorId);
            }
            for index in old_indices {
                old_loss_reasons[*index] = Some(ReferenceLossReason::AmbiguousAuthorId);
            }
        }
    }
    for indices in old_keys.values().filter(|indices| indices.len() > 1) {
        for index in indices {
            old_loss_reasons[*index] = Some(ReferenceLossReason::AmbiguousAuthorId);
        }
    }
}

fn match_semantic_keys(
    old: &[crate::StoredSemanticUnit],
    current: &[ExtractedSemanticUnit],
    old_used: &mut [bool],
    old_loss_reasons: &mut [Option<ReferenceLossReason>],
    current_refs: &mut [Option<ElementRef>],
    current_reasons: &mut [Option<ReferenceAssignmentReason>],
) {
    let old_keys = buckets(old.iter().enumerate().filter_map(|(index, unit)| {
        (!old_used[index])
            .then(|| semantic_key(unit.semantic()))
            .flatten()
            .map(|key| (key, index))
    }));
    let current_keys = buckets(current.iter().enumerate().filter_map(|(index, unit)| {
        current_refs[index]
            .is_none()
            .then(|| semantic_key(unit))
            .flatten()
            .map(|key| (key, index))
    }));

    for (key, current_indices) in &current_keys {
        let old_indices = old_keys.get(key).map_or(&[][..], Vec::as_slice);
        if let ([current_index], [old_index]) = (current_indices.as_slice(), old_indices) {
            old_used[*old_index] = true;
            current_refs[*current_index] = Some(old[*old_index].reference());
            current_reasons[*current_index] =
                Some(ReferenceAssignmentReason::UniqueSemanticIdentity);
        } else if current_indices.len() > 1 || old_indices.len() > 1 {
            for index in current_indices {
                current_reasons[*index] =
                    Some(ReferenceAssignmentReason::AmbiguousSemanticIdentity);
            }
            for index in old_indices {
                old_loss_reasons[*index] = Some(ReferenceLossReason::AmbiguousSemanticIdentity);
            }
        }
    }
    for indices in old_keys.values().filter(|indices| indices.len() > 1) {
        for index in indices {
            old_loss_reasons[*index] = Some(ReferenceLossReason::AmbiguousSemanticIdentity);
        }
    }
}

fn buckets<K: Ord>(values: impl Iterator<Item = (K, usize)>) -> BTreeMap<K, Vec<usize>> {
    let mut result = BTreeMap::new();
    for (key, index) in values {
        result.entry(key).or_insert_with(Vec::new).push(index);
    }
    result
}

fn author_id(unit: &ExtractedSemanticUnit) -> Option<String> {
    match unit.author_id() {
        Property::Known(value) => Some(value.as_str().to_owned()),
        Property::NotApplicable | Property::Unknown(_) => None,
    }
}

fn semantic_key(unit: &ExtractedSemanticUnit) -> Option<SemanticKey> {
    if !matches!(unit.author_id(), Property::NotApplicable) || unit.role() == SemanticRole::Page {
        return None;
    }
    let name = match unit.name() {
        Property::Known(value) => Some(value.as_str().to_owned()),
        Property::NotApplicable | Property::Unknown(_) => None,
    };
    let destination = match unit.destination() {
        Property::Known(value) => Some(value.as_str().to_owned()),
        Property::NotApplicable | Property::Unknown(_) => None,
    };
    (name.is_some() || destination.is_some()).then_some(SemanticKey {
        role: unit.role(),
        name,
        destination,
    })
}

fn assignment_reason(unit: &ExtractedSemanticUnit) -> ReferenceAssignmentReason {
    match unit.author_id() {
        Property::Unknown(mawr_core::PropertyUnknownReason::Ambiguous) => {
            ReferenceAssignmentReason::AmbiguousAuthorId
        }
        Property::Unknown(_) => ReferenceAssignmentReason::NoStableIdentity,
        Property::Known(_) => ReferenceAssignmentReason::NewElement,
        Property::NotApplicable if semantic_key(unit).is_some() => {
            ReferenceAssignmentReason::NewElement
        }
        Property::NotApplicable => ReferenceAssignmentReason::NoStableIdentity,
    }
}

fn unmatched_loss_reason(unit: &ExtractedSemanticUnit) -> ReferenceLossReason {
    match unit.author_id() {
        Property::Unknown(mawr_core::PropertyUnknownReason::Ambiguous) => {
            ReferenceLossReason::AmbiguousAuthorId
        }
        Property::NotApplicable | Property::Known(_) | Property::Unknown(_) => {
            ReferenceLossReason::RemovedOrIdentityChanged
        }
    }
}

fn allocate_reference(
    session: SessionId,
    sequence: &mut u64,
) -> Result<ElementRef, OperationFailure> {
    let value = u32::try_from(*sequence).map_err(|_| OperationFailure::ResourceLimit {
        resource: ResourceKind::StateRetention,
        configured_limit: NonZeroU64::new(u64::from(u32::MAX)).expect("u32::MAX is non-zero"),
    })?;
    let reference = ElementRef::new(session, value).map_err(OperationFailure::InvalidInput)?;
    *sequence = sequence.saturating_add(1);
    Ok(reference)
}

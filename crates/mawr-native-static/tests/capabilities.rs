use mawr_core::{Capability, CapabilityStatus, FailureClass, SessionId, UnsupportedReason};
use mawr_native_static::{NativeStaticConfig, NativeStaticEngine};

#[test]
fn capability_report_is_truthful_for_m2() {
    let engine = NativeStaticEngine::new(NativeStaticConfig::default());
    assert!(matches!(
        engine.capabilities().status(Capability::Http),
        CapabilityStatus::Limited(_)
    ));
    assert_eq!(
        engine.capabilities().status(Capability::JavaScript),
        &CapabilityStatus::Unsupported(UnsupportedReason::NotImplemented)
    );
    assert_eq!(
        engine
            .start_session(SessionId::new(1).unwrap())
            .engine_identity(),
        engine.identity()
    );
}

#[test]
fn transport_has_no_vendor_fallback_failure_class() {
    let engine = NativeStaticEngine::new(NativeStaticConfig::default());
    let session = engine.start_session(SessionId::new(1).unwrap());
    assert_eq!(
        session.unsupported(Capability::JavaScript).class(),
        FailureClass::UnsupportedCapability
    );
}

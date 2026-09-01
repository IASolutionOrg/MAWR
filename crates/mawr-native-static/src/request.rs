use mawr_core::{AbsoluteUrl, BoundedText, SensitiveText, ValidationError};

use crate::ConfigError;

const MAX_FORM_FIELDS: usize = 256;
const MAX_FORM_NAME_BYTES: usize = 256;
const MAX_FORM_VALUE_BYTES: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RequestMethod {
    Get,
    Head,
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FormMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormField {
    name: BoundedText<MAX_FORM_NAME_BYTES>,
    value: SensitiveText<MAX_FORM_VALUE_BYTES>,
}

impl FormField {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self {
            name: BoundedText::new(name, "form_field_name")?,
            value: SensitiveText::new(value, "form_field_value")?,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub(crate) fn value(&self) -> &str {
        self.value.expose_secret()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormSubmission {
    fields: Vec<FormField>,
}

impl FormSubmission {
    pub fn new(fields: Vec<FormField>) -> Result<Self, ConfigError> {
        if fields.len() > MAX_FORM_FIELDS {
            return Err(ConfigError::TooManyValues);
        }
        Ok(Self { fields })
    }

    #[must_use]
    pub fn fields(&self) -> &[FormField] {
        &self.fields
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RequestPayload {
    None,
    Form {
        method: FormMethod,
        submission: FormSubmission,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationRequest {
    destination: AbsoluteUrl,
    payload: RequestPayload,
    head: bool,
}

impl NavigationRequest {
    #[must_use]
    pub const fn get(destination: AbsoluteUrl) -> Self {
        Self {
            destination,
            payload: RequestPayload::None,
            head: false,
        }
    }

    #[must_use]
    pub const fn head(destination: AbsoluteUrl) -> Self {
        Self {
            destination,
            payload: RequestPayload::None,
            head: true,
        }
    }

    #[must_use]
    pub const fn submit_form(
        destination: AbsoluteUrl,
        method: FormMethod,
        submission: FormSubmission,
    ) -> Self {
        Self {
            destination,
            payload: RequestPayload::Form { method, submission },
            head: false,
        }
    }

    #[must_use]
    pub const fn destination(&self) -> &AbsoluteUrl {
        &self.destination
    }

    #[must_use]
    pub const fn method(&self) -> RequestMethod {
        match self.payload {
            RequestPayload::None if self.head => RequestMethod::Head,
            RequestPayload::None => RequestMethod::Get,
            RequestPayload::Form {
                method: FormMethod::Get,
                ..
            } => RequestMethod::Get,
            RequestPayload::Form {
                method: FormMethod::Post,
                ..
            } => RequestMethod::Post,
        }
    }

    pub(crate) const fn form(&self) -> Option<(FormMethod, &FormSubmission)> {
        match &self.payload {
            RequestPayload::None => None,
            RequestPayload::Form { method, submission } => Some((*method, submission)),
        }
    }
}

#[cfg(test)]
mod tests {
    use mawr_core::AbsoluteUrl;

    use super::{FormField, FormMethod, FormSubmission, NavigationRequest};

    #[test]
    fn form_debug_redacts_values() {
        let field = FormField::new("password", "secret-value").unwrap();
        let request = NavigationRequest::submit_form(
            AbsoluteUrl::new("https://example.test/login").unwrap(),
            FormMethod::Post,
            FormSubmission::new(vec![field]).unwrap(),
        );

        assert!(!format!("{request:?}").contains("secret-value"));
    }
}

/// Type definitions for field specifications

#[derive(Debug, Clone)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    Letters,             // 'l' - matches only letters
    Word,                // 'w' - matches word characters (letters, digits, underscore)
    NonLetters,          // 'W' - matches non-letter characters
    NonWhitespace,       // 'S' - matches non-whitespace characters
    NonDigits,           // 'D' - matches non-digit characters
    NumberWithThousands, // 'n' - numbers with thousands separators
    Scientific,          // 'e' - scientific notation
    GeneralNumber,       // 'g' - general number (int or float)
    Percentage,          // '%' - percentage
    DateTimeISO,         // 'ti' - ISO 8601 datetime format
    DateTimeRFC2822,     // 'te' - RFC2822 email format
    DateTimeGlobal,      // 'tg' - Global (day/month) format
    DateTimeUS,          // 'ta' - US (month/day) format
    DateTimeCtime,       // 'tc' - ctime() format
    DateTimeHTTP,        // 'th' - HTTP log format
    DateTimeTime,        // 'tt' - Time format
    DateTimeSystem,      // 'ts' - Linux system log format
    DateTimeStrftime,    // For %Y-%m-%d style patterns
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
    pub name: Option<String>,
    pub field_type: FieldType,
    pub width: Option<usize>,
    pub precision: Option<usize>,
    pub alignment: Option<char>, // '<', '>', '^', '='
    pub sign: Option<char>,      // '+', '-', ' '
    pub fill: Option<char>,
    pub zero_pad: bool,
    pub strftime_format: Option<String>, // For strftime-style patterns
    pub original_type_char: Option<char>, // Original type character (e.g., 'b', 'o', 'x' for binary/octal/hex)
}

impl Default for FieldSpec {
    fn default() -> Self {
        Self {
            name: None,
            field_type: FieldType::String,
            width: None,
            precision: None,
            alignment: None,
            sign: None,
            fill: None,
            zero_pad: false,
            strftime_format: None,
            original_type_char: None,
        }
    }
}

impl FieldSpec {
    pub fn new() -> Self {
        Self::default()
    }

    /// True for `{}` / `{name}` with no width, precision, or alignment (the plain
    /// string branch in the regex builder). Used for empty-input `parse`
    /// handling (formatparse#16) without changing search/findall semantics.
    pub fn is_default_unconstrained_string(&self) -> bool {
        matches!(self.field_type, FieldType::String)
            && self.width.is_none()
            && self.precision.is_none()
            && self.alignment.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_spec_default() {
        let spec = FieldSpec::default();
        assert!(spec.name.is_none());
        assert!(matches!(spec.field_type, FieldType::String));
        assert!(spec.width.is_none());
        assert!(spec.precision.is_none());
        assert!(spec.alignment.is_none());
        assert!(spec.sign.is_none());
        assert!(spec.fill.is_none());
        assert!(!spec.zero_pad);
        assert!(spec.strftime_format.is_none());
        assert!(spec.original_type_char.is_none());
    }

    #[test]
    fn test_field_spec_new() {
        let spec = FieldSpec::new();
        assert!(matches!(spec.field_type, FieldType::String));
    }

    #[test]
    fn test_field_type_equality() {
        let t1 = FieldType::String;
        let t2 = FieldType::String;
        let t3 = FieldType::Integer;

        // Test pattern matching (since FieldType doesn't implement Eq)
        match (&t1, &t2) {
            (FieldType::String, FieldType::String) => (),
            _ => panic!("Should match"),
        }

        match (&t1, &t3) {
            (FieldType::String, FieldType::Integer) => (),
            _ => panic!("Should be different"),
        }
    }

    #[test]
    fn test_field_type_custom() {
        let custom = FieldType::Custom("MyType".to_string());
        match custom {
            FieldType::Custom(name) => assert_eq!(name, "MyType"),
            _ => panic!("Should be Custom type"),
        }
    }

    #[test]
    fn test_field_spec_with_all_fields() {
        let spec = FieldSpec {
            name: Some("test".to_string()),
            field_type: FieldType::Integer,
            width: Some(10),
            precision: Some(2),
            alignment: Some('>'),
            sign: Some('+'),
            fill: Some('x'),
            zero_pad: true,
            strftime_format: Some("%Y-%m-%d".to_string()),
            original_type_char: Some('d'),
        };

        assert_eq!(spec.name, Some("test".to_string()));
        assert!(matches!(spec.field_type, FieldType::Integer));
        assert_eq!(spec.width, Some(10));
        assert_eq!(spec.precision, Some(2));
        assert_eq!(spec.alignment, Some('>'));
        assert_eq!(spec.sign, Some('+'));
        assert_eq!(spec.fill, Some('x'));
        assert!(spec.zero_pad);
        assert_eq!(spec.strftime_format, Some("%Y-%m-%d".to_string()));
        assert_eq!(spec.original_type_char, Some('d'));

        // Test cloning
        let cloned = spec.clone();
        assert_eq!(cloned.name, spec.name);
        assert_eq!(cloned.width, spec.width);
    }
}

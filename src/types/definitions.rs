/// Type definitions for field specifications

#[derive(Debug, Clone)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    Letters,      // 'l' - matches only letters
    Word,         // 'w' - matches word characters (letters, digits, underscore)
    NonLetters,   // 'W' - matches non-letter characters
    NonWhitespace,// 'S' - matches non-whitespace characters
    NonDigits,    // 'D' - matches non-digit characters
    NumberWithThousands, // 'n' - numbers with thousands separators
    Scientific,   // 'e' - scientific notation
    GeneralNumber,// 'g' - general number (int or float)
    Percentage,   // '%' - percentage
    DateTimeISO,  // 'ti' - ISO 8601 datetime format
    DateTimeRFC2822, // 'te' - RFC2822 email format
    DateTimeGlobal, // 'tg' - Global (day/month) format
    DateTimeUS,   // 'ta' - US (month/day) format
    DateTimeCtime, // 'tc' - ctime() format
    DateTimeHTTP, // 'th' - HTTP log format
    DateTimeTime, // 'tt' - Time format
    DateTimeSystem, // 'ts' - Linux system log format
    DateTimeStrftime, // For %Y-%m-%d style patterns
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
}


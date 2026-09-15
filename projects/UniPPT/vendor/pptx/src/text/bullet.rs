//! Bullet format types for paragraph bullets.

/// Specifies the bullet format for a paragraph.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum BulletFormat {
    /// A character bullet (e.g. `'•'`, `'–'`).
    Character(char),
    /// An auto-numbered bullet (e.g. `"arabicPeriod"`, `"alphaLcParenR"`).
    AutoNumbered(String),
    /// A picture bullet via relationship ID (`<a:buBlip>`).
    Picture(String),
    /// No bullet (explicitly suppressed).
    None,
}

pub const NAME: &str = "FDE2E Photocraft";
pub const DEVELOPER: &str = "FDE2E";
pub const REPOSITORY: &str = "jixoai/photocraft";
pub const GITHUB: &str = "https://github.com/jixoai/photocraft";
pub const ISSUES: &str = "https://github.com/jixoai/photocraft/issues";
pub const RELEASES: &str = "https://github.com/jixoai/photocraft/releases";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_and_support_destinations_belong_to_the_downstream() {
        assert_eq!(NAME, "FDE2E Photocraft");
        assert_eq!(GITHUB, format!("https://github.com/{REPOSITORY}"));
        assert_eq!(ISSUES, format!("{GITHUB}/issues"));
        assert_eq!(RELEASES, format!("{GITHUB}/releases"));
        assert_eq!(DEVELOPER, "FDE2E");
    }
}

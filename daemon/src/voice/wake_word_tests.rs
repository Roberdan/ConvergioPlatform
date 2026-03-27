use super::wake_word::WakeWordDetector;

#[test]
fn detects_wake_word() {
    let mut wd = WakeWordDetector::new("convergio");
    assert!(wd.check("hey convergio how are you").unwrap());
}

#[test]
fn case_insensitive() {
    let mut wd = WakeWordDetector::new("convergio");
    assert!(wd.check("Hey CONVERGIO").unwrap());
}

#[test]
fn no_false_positive() {
    let mut wd = WakeWordDetector::new("convergio");
    assert!(!wd.check("the weather is nice today").unwrap());
}

#[test]
fn reset_clears() {
    let mut wd = WakeWordDetector::new("convergio");
    wd.check("convergio").unwrap();
    wd.reset();
    assert_eq!(wd.wake_word(), "convergio");
}

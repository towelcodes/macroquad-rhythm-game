use crate::db::{DatabaseController, DbError};

/// set up a new db controller in memory, and run migrations
fn setup() -> DatabaseController {
    let controller = DatabaseController::new(":memory:").expect("failed to open db");
    controller.perform_migrations().expect("migrations failed");
    controller
}

#[test]
pub fn migrations_apply_once() {
    let controller = DatabaseController::new(":memory:").unwrap();
    assert!(
        controller.perform_migrations().unwrap(),
        "first run should apply migrations"
    );
    assert!(
        !controller.perform_migrations().unwrap(),
        "second run should do nothing"
    );
}

#[test]
pub fn register_user_returns_id_and_token() {
    let mut controller = setup();
    let (uid, token) = controller
        .register_user("user1".into(), "pw1".into())
        .unwrap();
    assert_ne!(uid, 0);
    assert_ne!(token, [0u8; 32]);
}

#[test]
pub fn register_duplicate_username_fails() {
    let mut controller = setup();
    controller
        .register_user("user1".into(), "pw1".into())
        .unwrap();
    let err = controller
        .register_user("user1".into(), "pw2".into())
        .unwrap_err();
    assert!(matches!(err, DbError::AccountAlreadyExists));
}

#[test]
pub fn registered_users_get_distinct_ids() {
    let mut controller = setup();
    let (first, _) = controller
        .register_user("user1".into(), "pw1".into())
        .unwrap();
    let (second, _) = controller
        .register_user("user2".into(), "pw2".into())
        .unwrap();
    assert_ne!(first, second);
}

#[test]
pub fn login_with_correct_password_succeeds() {
    let mut controller = setup();
    controller
        .register_user("user1".into(), "pw1".into())
        .unwrap();
    controller.login_user("user1".into(), "pw1".into()).unwrap();
}

#[test]
pub fn login_with_wrong_password_fails() {
    let mut controller = setup();
    controller
        .register_user("user1".into(), "pw1".into())
        .unwrap();
    let err = controller
        .login_user("user1".into(), "wrong".into())
        .unwrap_err();
    assert!(matches!(err, DbError::IncorrectCredentials));
}

#[test]
pub fn login_unknown_user_fails() {
    let mut controller = setup();
    let err = controller
        .login_user("nobody".into(), "pw".into())
        .unwrap_err();
    assert!(matches!(err, DbError::IncorrectCredentials));
}

#[test]
pub fn logout_invalidates_token() {
    let mut controller = setup();
    let (uid, token) = controller
        .register_user("user1".into(), "pw1".into())
        .unwrap();
    controller.logout_user(uid, token).unwrap();

    // the token row is gone, so a second logout must be rejected
    let err = controller.logout_user(uid, token).unwrap_err();
    assert!(matches!(err, DbError::NotAuthorized));
}

#[test]
pub fn logout_with_someone_elses_token_is_unauthorized() {
    let mut controller = setup();
    let (user1, _) = controller
        .register_user("user1".into(), "pw1".into())
        .unwrap();
    let (_, user2_token) = controller
        .register_user("user2".into(), "pw2".into())
        .unwrap();

    let err = controller.logout_user(user1, user2_token).unwrap_err();
    assert!(matches!(err, DbError::NotAuthorized));
}

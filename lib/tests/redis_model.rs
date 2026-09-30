use redis_om::{RedisModel, RedisTransportValue};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[test]
fn basic_with_no_options() -> Result {
    #[derive(RedisTransportValue, RedisModel)]
    struct Account {
        id: String,
        first_name: String,
    }

    let mut account = Account {
        id: "".into(),
        first_name: "Joe".into(),
    };

    assert_eq!(Account::_prefix_key(), "Account");
    assert_eq!(account.id, "");
    account._set_pk("1234".to_string());
    assert_eq!(account.id, "1234");

    Ok(())
}

#[test]
fn infers_primary_key_from_explicit_serialized_name_before_rename_all() -> Result {
    #[derive(RedisTransportValue, RedisModel)]
    #[redis(rename_all = "SCREAMING_SNAKE_CASE")]
    struct Account {
        #[redis(rename(serialize = "id"))]
        account_id: String,
        first_name: String,
    }

    let mut account = Account {
        account_id: "1234".into(),
        first_name: "Joe".into(),
    };

    assert_eq!(Account::_prefix_key(), "Account");
    assert_eq!(account._get_pk(), "1234");
    account._set_pk("5678".to_string());
    assert_eq!(account._get_pk(), "5678");
    assert_eq!(account.account_id, "5678");

    Ok(())
}

#[test]
fn basic_with_custom_prefix_and_pk() -> Result {
    #[derive(RedisTransportValue, RedisModel)]
    #[redis(prefix_key = "info_details")]
    struct Details {
        #[redis(primary_key)]
        pk: String,
        city: String,
    }

    let mut details = Details {
        pk: "".into(),
        city: "Joe".into(),
    };

    assert_eq!(Details::_prefix_key(), "info_details");
    assert_eq!(details.pk, "");
    details._set_pk("1234".to_string());
    assert_eq!(details.pk, "1234");

    Ok(())
}

#[test]
fn accepts_legacy_key_as_prefix() -> Result {
    #[derive(RedisModel)]
    #[redis(key = "legacy_records")]
    struct Record {
        id: String,
    }

    assert_eq!(Record::_prefix_key(), "legacy_records");

    Ok(())
}

#[test]
fn recognizes_formatted_primary_keys_only_at_the_delimiter() -> Result {
    #[derive(RedisModel)]
    #[redis(prefix_key = "user")]
    struct User {
        id: String,
    }

    assert!(User::_is_pk_fmt("user:7"));
    assert_eq!(User::_fmt_pk("7"), "user:7");
    assert!(!User::_is_pk_fmt("username:7"));
    assert!(!User::_is_pk_fmt("user"));
    assert!(!User::_is_pk_fmt("7"));
    assert!(User::_is_pk_fmt("user:"));

    #[derive(RedisModel)]
    #[redis(prefix_key = "user:")]
    struct NamespacedUser {
        id: String,
    }

    assert!(NamespacedUser::_is_pk_fmt("user::7"));
    assert!(!NamespacedUser::_is_pk_fmt("user:7"));

    Ok(())
}

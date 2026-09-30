use redis_om::redis::{from_redis_value, Value};
use redis_om::redis::{FromRedisValue, ToRedisArgs};
use redis_om::RedisTransportValue;
use std::collections::HashMap;

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[test]
fn struct_with_no_options() -> Result {
    #[derive(RedisTransportValue)]
    struct Account {
        #[redis(primary_key)]
        first_name: String,
        last_name: String,
        interests: Vec<String>,
    }

    let account = Account {
        first_name: "Joe".into(),
        last_name: "Doe".into(),
        interests: vec!["Gaming".into(), "SandCasting".into(), "Writing".into()],
    };

    let serialized = account
        .to_redis_args()
        .into_iter()
        .map(|v| Value::Data(v))
        .collect::<Vec<_>>();
    let deserialized = Account::from_redis_value(&Value::Bulk(serialized))?;

    // Ensure that values are identical
    assert_eq!(account.first_name, deserialized.first_name);
    assert_eq!(account.last_name, deserialized.last_name);
    assert_eq!(account.interests, deserialized.interests);

    Ok(())
}

#[test]
fn struct_with_rename_all_option() -> Result {
    #[derive(RedisTransportValue)]
    #[redis(rename_all = "camelCase")]
    struct Account {
        #[redis(primary_key)]
        first_name: String,
        last_name: String,
    }

    let account = Account {
        first_name: "Joe".into(),
        last_name: "Doe".into(),
    };

    let serialized = Value::Bulk(
        account
            .to_redis_args()
            .into_iter()
            .map(|v| Value::Data(v))
            .collect::<Vec<_>>(),
    );

    // Ensure that values are identical
    let deserialized = Account::from_redis_value(&serialized)?;
    assert_eq!(account.first_name, deserialized.first_name);
    assert_eq!(account.last_name, deserialized.last_name);

    // Ensure that fields are renamed
    let account_map: HashMap<String, String> = from_redis_value(&serialized)?;
    assert_eq!(account_map["firstName"], account.first_name);
    assert_eq!(account_map["lastName"], account.last_name);

    Ok(())
}

#[test]
fn struct_with_directional_rename_and_aliases() -> Result {
    #[derive(RedisTransportValue)]
    #[redis(rename_all = "camelCase")]
    struct Account {
        #[redis(
            rename(
                serialize = "outgoing_name",
                deserialize = "incoming_name",
                deserialize = "historic_name"
            ),
            alias = "legacy_name"
        )]
        first_name: String,
        #[redis(alias = "oldLastName", alias = "olderLastName")]
        last_name: String,
    }

    let account = Account {
        first_name: "Joe".into(),
        last_name: "Doe".into(),
    };

    let serialized = Value::Bulk(
        account
            .to_redis_args()
            .into_iter()
            .map(Value::Data)
            .collect::<Vec<_>>(),
    );
    let account_map: HashMap<String, String> = from_redis_value(&serialized)?;
    assert_eq!(account_map["outgoing_name"], account.first_name);
    assert_eq!(account_map["lastName"], account.last_name);
    assert!(!account_map.contains_key("firstName"));

    for first_name_key in ["incoming_name", "historic_name", "legacy_name"] {
        let value = Value::Bulk(vec![
            Value::Data(first_name_key.as_bytes().to_vec()),
            Value::Data(b"Joe".to_vec()),
            Value::Data(b"oldLastName".to_vec()),
            Value::Data(b"Doe".to_vec()),
        ]);
        let deserialized = Account::from_redis_value(&value)?;
        assert_eq!(deserialized.first_name, account.first_name);
        assert_eq!(deserialized.last_name, account.last_name);
    }

    for last_name_key in ["oldLastName", "olderLastName", "lastName"] {
        let value = Value::Bulk(vec![
            Value::Data(b"incoming_name".to_vec()),
            Value::Data(b"Joe".to_vec()),
            Value::Data(last_name_key.as_bytes().to_vec()),
            Value::Data(b"Doe".to_vec()),
        ]);
        let deserialized = Account::from_redis_value(&value)?;
        assert_eq!(deserialized.first_name, account.first_name);
        assert_eq!(deserialized.last_name, account.last_name);
    }

    let inherited_names = Value::Bulk(vec![
        Value::Data(b"firstName".to_vec()),
        Value::Data(b"Joe".to_vec()),
        Value::Data(b"lastName".to_vec()),
        Value::Data(b"Doe".to_vec()),
    ]);
    assert!(Account::from_redis_value(&inherited_names).is_err());

    Ok(())
}

#[test]
fn struct_can_skip_serializing_a_field() -> Result {
    #[derive(RedisTransportValue)]
    struct Account {
        visible: String,
        #[redis(skip_serializing)]
        internal: String,
    }

    let account = Account {
        visible: "shown".into(),
        internal: "hidden".into(),
    };
    assert_eq!(
        account.to_redis_args(),
        vec![b"visible".to_vec(), b"shown".to_vec()]
    );

    let value = Value::Bulk(vec![
        Value::Data(b"visible".to_vec()),
        Value::Data(b"shown".to_vec()),
        Value::Data(b"internal".to_vec()),
        Value::Data(b"hidden".to_vec()),
    ]);
    let decoded = Account::from_redis_value(&value)?;
    assert_eq!(decoded.visible, "shown");
    assert_eq!(decoded.internal, "hidden");

    Ok(())
}

#[test]
fn enum_with_no_options() -> Result {
    #[derive(Debug, RedisTransportValue, PartialEq)]
    enum State {
        On,
        Off,
    }

    let state = State::On;

    let serialized = state.to_redis_args().first().unwrap().to_vec();
    let deserialized = State::from_redis_value(&Value::Data(serialized))?;

    // Ensure that values are identical
    assert_eq!(deserialized, state);

    Ok(())
}

#[test]
fn enum_with_rename_all_option() -> Result {
    #[derive(Debug, RedisTransportValue, PartialEq)]
    #[redis(rename_all = "kebab-case")]
    enum State {
        OnDevice,
        OffDevice,
    }

    let state = State::OffDevice;

    let serialized = state.to_redis_args().first().unwrap().to_vec();
    let data = Value::Data(serialized.clone());
    let stringified = String::from_utf8(serialized)?;
    let deserialized = State::from_redis_value(&data)?;

    // Ensure that value is lowercase
    assert_eq!(stringified.as_str(), "off-device");
    // Ensure that values are identical
    assert_eq!(deserialized, state);

    Ok(())
}

#[test]
fn enum_with_aliases() -> Result {
    #[derive(Debug, RedisTransportValue, PartialEq)]
    enum State {
        #[redis(rename = "active", alias = "enabled", alias = "ready")]
        Active,
    }

    assert_eq!(State::Active.to_redis_args().first().unwrap(), b"active");

    for value in [b"active".as_slice(), b"enabled", b"ready"] {
        assert_eq!(
            State::from_redis_value(&Value::Data(value.to_vec()))?,
            State::Active
        );
    }

    Ok(())
}

#[test]
fn enum_can_skip_deserializing_a_variant() -> Result {
    #[derive(Debug, RedisTransportValue, PartialEq)]
    enum State {
        #[redis(skip_deserializing)]
        Hidden,
        Visible,
    }

    assert_eq!(State::Hidden.to_redis_args().first().unwrap(), b"Hidden");
    assert_eq!(
        State::from_redis_value(&Value::Data(b"Visible".to_vec()))?,
        State::Visible
    );
    assert!(State::from_redis_value(&Value::Data(b"Hidden".to_vec())).is_err());

    Ok(())
}

#[test]
fn dotted_scalar_key_round_trips_alongside_a_collection() -> Result {
    #[derive(Debug, PartialEq, RedisTransportValue)]
    struct Record {
        #[redis(rename = "account.balance")]
        balance: String,
        items: Vec<String>,
    }

    let record = Record {
        balance: "steady".into(),
        items: vec!["first".into(), "second".into()],
    };
    let arguments = record.to_redis_args();
    assert_eq!(arguments[0], b"account.balance".to_vec());
    assert_eq!(arguments[2], b"items.0".to_vec());
    assert_eq!(arguments[4], b"items.1".to_vec());

    let value = Value::Bulk(arguments.into_iter().map(Value::Data).collect());
    assert_eq!(Record::from_redis_value(&value)?, record);

    let one_item_record = Record {
        balance: "steady".into(),
        items: vec!["only".into()],
    };
    let one_item_arguments = one_item_record.to_redis_args();
    assert_eq!(one_item_arguments[2], b"items".to_vec());
    let one_item_value = Value::Bulk(one_item_arguments.into_iter().map(Value::Data).collect());
    assert_eq!(Record::from_redis_value(&one_item_value)?, one_item_record);

    #[derive(Debug, PartialEq, RedisTransportValue)]
    struct ScalarKeys {
        root: std::string::String,
        #[redis(rename = "root.0")]
        first: std::string::String,
    }

    let scalar_keys = ScalarKeys {
        root: "root".into(),
        first: "first".into(),
    };
    let scalar_key_value = Value::Bulk(
        scalar_keys
            .to_redis_args()
            .into_iter()
            .map(Value::Data)
            .collect(),
    );
    assert_eq!(
        ScalarKeys::from_redis_value(&scalar_key_value)?,
        scalar_keys
    );

    Ok(())
}

#[test]
fn supports_all_rename_all_rules() -> Result {
    macro_rules! assert_rename_all {
        ($struct_name:ident, $enum_name:ident, $rule:literal, $field:literal, $variant:literal) => {
            #[derive(RedisTransportValue)]
            #[redis(rename_all = $rule)]
            struct $struct_name {
                very_tasty: String,
            }

            #[derive(Debug, RedisTransportValue, PartialEq)]
            #[redis(rename_all = $rule)]
            enum $enum_name {
                VeryTasty,
            }

            let value = $struct_name {
                very_tasty: "value".into(),
            };
            assert_eq!(
                value.to_redis_args().first().unwrap().as_slice(),
                $field.as_bytes()
            );
            assert_eq!(
                $enum_name::VeryTasty
                    .to_redis_args()
                    .first()
                    .unwrap()
                    .as_slice(),
                $variant.as_bytes()
            );
        };
    }

    assert_rename_all!(
        LowercaseField,
        LowercaseVariant,
        "lowercase",
        "very_tasty",
        "verytasty"
    );
    assert_rename_all!(
        UppercaseField,
        UppercaseVariant,
        "UPPERCASE",
        "VERY_TASTY",
        "VERYTASTY"
    );
    assert_rename_all!(
        PascalField,
        PascalVariant,
        "PascalCase",
        "VeryTasty",
        "VeryTasty"
    );
    assert_rename_all!(
        CamelField,
        CamelVariant,
        "camelCase",
        "veryTasty",
        "veryTasty"
    );
    assert_rename_all!(
        SnakeField,
        SnakeVariant,
        "snake_case",
        "very_tasty",
        "very_tasty"
    );
    assert_rename_all!(
        ScreamingSnakeField,
        ScreamingSnakeVariant,
        "SCREAMING_SNAKE_CASE",
        "VERY_TASTY",
        "VERY_TASTY"
    );
    assert_rename_all!(
        KebabField,
        KebabVariant,
        "kebab-case",
        "very-tasty",
        "very-tasty"
    );
    assert_rename_all!(
        ScreamingKebabField,
        ScreamingKebabVariant,
        "SCREAMING-KEBAB-CASE",
        "VERY-TASTY",
        "VERY-TASTY"
    );

    Ok(())
}

#[test]
fn enum_struct_compo() -> Result {
    #[derive(Debug, RedisTransportValue, PartialEq)]
    enum AccountKind {
        #[redis(alias = "owner")]
        Admin,
        Shopper,
    }

    #[derive(RedisTransportValue)]
    #[redis(rename_all = "camelCase")]
    struct Account {
        #[redis(primary_key)]
        first_name: String,
        last_name: String,
        #[redis(rename = "accountKind")]
        kind: AccountKind,
    }

    let account = Account {
        first_name: "Joe".into(),
        last_name: "Doe".into(),
        kind: AccountKind::Shopper,
    };

    let serialized = Value::Bulk(
        account
            .to_redis_args()
            .into_iter()
            .map(|v| Value::Data(v))
            .collect::<Vec<_>>(),
    );

    // Ensure that values are identical
    let deserialized = Account::from_redis_value(&serialized)?;
    assert_eq!(account.first_name, deserialized.first_name);
    assert_eq!(account.last_name, deserialized.last_name);
    assert_eq!(account.kind, deserialized.kind);

    // Ensure that fields are renamed
    let account_map: HashMap<String, String> = from_redis_value(&serialized)?;
    assert_eq!(account_map["firstName"], account.first_name);
    assert_eq!(account_map["lastName"], account.last_name);
    assert_eq!(account_map["accountKind"], "Shopper");

    Ok(())
}

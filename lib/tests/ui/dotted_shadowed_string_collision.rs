use redis_om::RedisTransportValue;

type String = Vec<std::string::String>;

#[derive(RedisTransportValue)]
struct Record {
    items: String,
    #[redis(rename = "items.0")]
    first_item_label: std::string::String,
}

fn main() {}

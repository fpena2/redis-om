use redis_om::RedisTransportValue;

type Option<T> = Vec<T>;

#[derive(RedisTransportValue)]
struct Record {
    items: Option<std::string::String>,
    #[redis(rename = "items.0")]
    first_item_label: std::string::String,
}

fn main() {}

use redis_om::RedisTransportValue;

#[derive(RedisTransportValue)]
struct Record {
    #[redis(rename = "items.0")]
    first_item: std::string::String,
    items: Vec<String>,
}

fn main() {}

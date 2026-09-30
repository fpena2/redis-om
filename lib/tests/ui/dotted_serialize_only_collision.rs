use redis_om::RedisTransportValue;

#[derive(RedisTransportValue)]
struct Record {
    items: Vec<String>,
    #[redis(rename(serialize = "items.0", deserialize = "first_item_label"))]
    first_item_label: String,
}

fn main() {}

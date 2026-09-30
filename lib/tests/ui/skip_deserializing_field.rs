use redis_om::RedisTransportValue;

#[derive(RedisTransportValue)]
struct Record {
    #[redis(skip_deserializing)]
    required: String,
}

fn main() {}

use redis_om::RedisTransportValue;

mod custom {
    pub type String = Vec<std::string::String>;
}

#[derive(RedisTransportValue)]
struct Record {
    custom_value: custom::String,
    #[redis(rename = "custom_value.0")]
    first_custom_value: std::string::String,
}

fn main() {}

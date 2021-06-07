use crate::entities::command::Command;
use crate::entities::redis_element::RedisElement;
use crate::entities::redis_element::RedisElement::List;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Redis {
    db: HashMap<String, RedisElement>,
}

impl Redis {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let map = HashMap::new();

        Self { db: map }
    }

    #[allow(dead_code)]
    pub fn execute(&mut self, command: Command) -> Result<String, String> {
        match command {
            Command::Ping => Ok("PONG".to_string()),
            Command::Copy {
                key_origin,
                key_destination,
            } => self.copy_method(key_origin, key_destination),
            Command::Get { key } => self.get_method(key),
            Command::Set { key, value } => Ok(self.set_method(key, value)),
            Command::Del { keys } => Ok(self.del_method(keys)),
            Command::Exists { keys } => Ok(self.exists_method(keys)),
            Command::Rename {
                key_origin,
                key_destination,
            } => self.rename_method(key_origin, key_destination),
            Command::Incrby { key, increment } => self.incrby_method(key, increment),
            Command::Getdel { key } => self.getdel_method(key),
            Command::Append { key, value } => Ok(self.append_method(key, value)),
            Command::Dbsize => Ok(self.db.len().to_string()),

            Command::Lindex { key, index } => self.lindex_method(key, index),
            Command::Llen { key } => self.llen_method(key),
            Command::Lpush { key, value } => self.lpush_method(key, value),
            Command::Sadd {key, values} => self.sadd_method(key, values),
        }
    }

    #[allow(dead_code)]
    fn copy_method(
        &mut self,
        key_origin: String,
        key_destination: String,
    ) -> Result<String, String> {
        // TODO: no debería usar el metodo SET, si se estan copiando valores deberia mantenerse el tipo de elemento (String, Set, List)

        match self.get_method(key_origin) {
            Ok(value) => Ok(self.set_method(key_destination, value)),
            Err(_) => Err("Not Found".to_string()),
        }
    }

    #[allow(dead_code)]
    fn get_method(&mut self, key: String) -> Result<String, String> {
        // TODO: deberia devolver NIL si no existe el elemento

        match self.db.get(key.as_str()) {
            Some(return_value) => match return_value {
                RedisElement::String(_) => Ok(return_value.to_string()),
                _ => Err("Not string".to_string()),
            },
            None => Err("Not Found".to_string()),
        }
    }

    #[allow(dead_code)]
    fn set_method(&mut self, key: String, value: String) -> String {
        self.db.insert(key, RedisElement::String(value));

        "Ok".to_string()
    }

    #[allow(dead_code)]
    fn incrby_method(&mut self, key: String, increment: u32) -> Result<String, String> {
        match self.get_method(key.clone()) {
            Ok(return_value) => {
                let my_int: Result<u32, _> = return_value.parse();
                if my_int.is_err() {
                    return Err("ERR value is not an integer or out of range".to_string());
                }

                let my_int = my_int.unwrap() + increment;
                Ok(self.set_method(key, my_int.to_string()))
            }
            Err(_) => Ok(self.set_method(key, increment.to_string())),
        }
    }

    #[allow(dead_code)]
    fn getdel_method(&mut self, key: String) -> Result<String, String> {
        match self.get_method(key.clone()) {
            Ok(return_value) => {
                self.db.remove(key.as_str());
                Ok(return_value)
            }
            Err(_) => Err("Not Found".to_string()),
        }
    }

    #[allow(dead_code)]
    fn del_method(&mut self, keys: Vec<String>) -> String {
        let mut count = 0;
        for key in keys.iter() {
            if self.db.remove(key.as_str()).is_some() {
                count += 1;
            }
        }

        count.to_string()
    }

    #[allow(dead_code)]
    fn append_method(&mut self, key: String, value: String) -> String {
        //TODO: chequar si el valor es string antes de hacer el append

        match self.get_method(key.clone()) {
            Ok(return_value) => {
                let value = return_value + value.as_str();

                self.set_method(key, value)
            }
            Err(_) => self.set_method(key, value),
        }
    }

    fn exists_method(&mut self, keys: Vec<String>) -> String {
        let mut count = 0;
        for key in keys.iter() {
            if self.db.contains_key(key.as_str()) {
                count += 1;
            }
        }
        count.to_string()
    }

    fn rename_method(
        &mut self,
        key_origin: String,
        key_destination: String,
    ) -> Result<String, String> {
        match self.getdel_method(key_origin) {
            Ok(value) => Ok(self.set_method(key_destination, value)),
            Err(msg) => Err(msg),
        }
    }

    fn lindex_method(&mut self, key: String, index: i32) -> Result<String, String> {
        match self.db.get_mut(key.as_str()) {
            Some(value) => match value {
                RedisElement::List(value) => {
                    let len_value = value.len() as i32;
                    let mut position: i32 = index;

                    if index < 0 {
                        position = index + len_value;
                    }

                    match value.get(position as usize) {
                        Some(saved_value) => Ok(saved_value.to_string()),
                        None => Ok("nil".to_string()),
                    }
                }
                _ => Err(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                ),
            },
            None => Ok("nil".to_string()),
        }
    }

    fn llen_method(&mut self, key: String) -> Result<String, String> {
        match self.db.get_mut(key.as_str()) {
            Some(value) => match value {
                RedisElement::List(value) => Ok(value.len().to_string()),
                _ => Err(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                ),
            },
            None => Ok("0".to_string()),
        }
    }

    fn lpush_method(&mut self, key: String, values: Vec<String>) -> Result<String, String> {
        let mut redis_element: Vec<String> = values;
        redis_element.reverse();

        match self.db.get_mut(key.as_str()) {
            Some(value) => match value {
                RedisElement::List(value) => {
                    let saved_vector = value.clone();
                    redis_element.extend(saved_vector);
                    self.db
                        .insert(key, RedisElement::List(redis_element.clone()));

                    Ok(redis_element.len().to_string())
                }
                _ => Err(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                ),
            },
            None => {
                self.db.insert(key, List(redis_element.clone()));

                Ok(redis_element.len().to_string())
            }
        }
    }

    fn sadd_method(&mut self, key: String, values: HashSet<String>) -> Result<String, String> {
        match self.db.get_mut(key.as_str()) {
            Some(value) => match value {
                RedisElement::Set(value) => {
                    let mut set = value.clone();
                    let start_set_len = set.clone().len();
                    set.extend(values.clone());
                    let final_set_len = set.clone().len();
                    self.db.insert(key, RedisElement::Set(set));

                    Ok((final_set_len - start_set_len).to_string())
                }
                _ => Err("WRONGTYPE A hashset data type expected".to_string())
            }
            None => {
                self.db.insert(key, RedisElement::Set(values.clone()));
                Ok(values.clone().len().to_string())
            }
        }
    }
}

#[allow(unused_imports)]
mod test {
    #[allow(unused_imports)]
    use crate::entities::command::Command;
    use crate::service::redis::Redis;
    use std::collections::HashSet;

    #[test]
    fn test_set_element_and_get_the_same() {
        let mut redis: Redis = Redis::new();

        let value: String = "value".to_string();
        let key: String = "hola".to_string();

        let _set = redis.execute(Command::Set { key, value });

        let key: String = "hola".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });

        assert_eq!("value".to_string(), get.unwrap().to_string());
    }

    #[test]
    fn test_set_element_twice_and_get_the_last_set() {
        let mut redis: Redis = Redis::new();

        let key: String = "hola".to_string();
        let value: String = "chau".to_string();

        let _set = redis.execute(Command::Set { key, value });

        let key: String = "hola".to_string();
        let value: String = "test".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "hola".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });

        assert_eq!("test".to_string(), get.unwrap().to_string());
    }

    #[test]
    fn test_get_element_not_found() {
        let mut redis: Redis = Redis::new();

        let key = "hola".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });

        assert!(get.is_err());
    }

    #[test]
    fn test_get_element_fail_if_is_not_strinng() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key: String = "key".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });

        assert!(get.is_err());
    }

    #[test]
    fn test_ping_retunrs_pong() {
        let mut redis: Redis = Redis::new();

        let ping: Result<String, String> = redis.execute(Command::Ping);

        assert_eq!("PONG".to_string(), ping.unwrap().to_string());
    }

    #[test]
    fn test_incrby_with_2_as_value() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value: String = "1".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "key".to_string();
        let increment: u32 = 1;
        let _incrby = redis.execute(Command::Incrby { key, increment });

        let key: String = "key".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });

        let key: String = "key".to_string();
        let increment: u32 = 2;
        let _incrby = redis.execute(Command::Incrby { key, increment });

        let key: String = "key".to_string();
        let second_get: Result<String, String> = redis.execute(Command::Get { key });

        assert_eq!("2".to_string(), get.unwrap().to_string());
        assert_eq!("4".to_string(), second_get.clone().unwrap().to_string());
        assert_ne!("10".to_string(), second_get.unwrap().to_string());
    }

    #[test]
    fn test_incrby_value_err_initial_value_string() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value: String = "hola".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "key".to_string();
        let increment: u32 = 1;
        let incrby = redis.execute(Command::Incrby { key, increment });

        assert!(incrby.is_err());
    }

    #[test]
    fn test_incrby_not_saved_value() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let increment: u32 = 1;
        let _incrby = redis.execute(Command::Incrby { key, increment });

        let key: String = "key".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });

        let key: String = "key".to_string();
        let second_get: Result<String, String> = redis.execute(Command::Get { key });

        assert_eq!("1".to_string(), get.unwrap().to_string());
        assert_ne!("10".to_string(), second_get.unwrap().to_string());
    }

    #[test]
    fn test_set_element_and_getdel() {
        let mut redis: Redis = Redis::new();

        let value: String = "value".to_string();
        let key: String = "key".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "key".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });

        let key: String = "key".to_string();
        let getdel: Result<String, String> = redis.execute(Command::Getdel { key });

        assert_eq!("value".to_string(), get.unwrap().to_string());
        assert_eq!("value".to_string(), getdel.unwrap().to_string());

        let key: String = "key".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });
        assert!(get.is_err());
    }

    #[test]
    fn test_getdel_without_previews_saving_err() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let getdel: Result<String, String> = redis.execute(Command::Getdel { key });
        assert!(getdel.is_err());
    }

    #[test]
    fn test_dbsize() {
        let mut redis: Redis = Redis::new();

        let dbsize: Result<String, String> = redis.execute(Command::Dbsize);
        assert_eq!("0".to_string(), dbsize.unwrap().to_string());

        let value: String = "value".to_string();
        let key: String = "key".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let dbsize: Result<String, String> = redis.execute(Command::Dbsize);
        assert_eq!("1".to_string(), dbsize.unwrap().to_string());

        let key: String = "key".to_string();
        let _getdel: Result<String, String> = redis.execute(Command::Getdel { key });

        let dbsize: Result<String, String> = redis.execute(Command::Dbsize);
        assert_eq!("0".to_string(), dbsize.unwrap().to_string());
    }

    #[test]
    fn test_set_element_and_del() {
        let mut redis: Redis = Redis::new();

        let value: String = "value".to_string();
        let key: String = "key".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let keys = vec!["key".to_string()];
        let del: Result<String, String> = redis.execute(Command::Del { keys });
        assert_eq!("1".to_string(), del.unwrap().to_string());

        let key: String = "key".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });
        assert!(get.is_err());
    }

    #[test]
    fn test_set_two_elements_and_del_both() {
        let mut redis: Redis = Redis::new();

        let value: String = "value".to_string();
        let key: String = "key1".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let value: String = "value".to_string();
        let key: String = "key2".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let keys = vec!["key1".to_string(), "key2".to_string()];
        let del: Result<String, String> = redis.execute(Command::Del { keys });

        assert_eq!("2".to_string(), del.unwrap().to_string());
    }

    #[test]
    fn test_append_adds_word() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value: String = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "key".to_string();
        let value: String = " appended".to_string();
        let _append = redis.execute(Command::Append { key, value });

        let key: String = "key".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });
        assert_eq!("value appended".to_string(), get.unwrap());
    }

    #[test]
    fn test_append_on_non_existent_key() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value: String = " appended".to_string();
        let _append = redis.execute(Command::Append { key, value });

        let key: String = "key".to_string();
        let get: Result<String, String> = redis.execute(Command::Get { key });

        assert_eq!(" appended".to_string(), get.unwrap());
    }

    #[test]
    fn test_set_two_elements_and_check_exists_equal_2() {
        let mut redis: Redis = Redis::new();

        let key: String = "key1".to_string();
        let value: String = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "key2".to_string();
        let value: String = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let keys = vec!["key1".to_string(), "key2".to_string()];
        let exists: Result<String, String> = redis.execute(Command::Exists { keys });
        assert_eq!("2".to_string(), exists.unwrap().to_string());

        let keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
        let exists: Result<String, String> = redis.execute(Command::Exists { keys });
        assert_eq!("2".to_string(), exists.unwrap().to_string());
    }

    #[test]
    fn test_set_two_elements_and_copy() {
        let mut redis: Redis = Redis::new();

        let key: String = "key1".to_string();
        let value: String = "value1".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "key2".to_string();
        let value: String = "value2".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "key2".to_string();
        let get = redis.execute(Command::Get { key });
        assert_eq!("value2".to_string(), get.unwrap().to_string());

        let key_origin: String = "key1".to_string();
        let key_destination: String = "key2".to_string();
        let _copy = redis.execute(Command::Copy {
            key_destination,
            key_origin,
        });

        let key: String = "key2".to_string();
        let get = redis.execute(Command::Get { key });
        assert_eq!("value1".to_string(), get.unwrap().to_string());
    }

    #[test]
    fn test_set_and_rename() {
        let mut redis: Redis = Redis::new();

        let key: String = "key1".to_string();
        let value: String = "value1".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key_origin: String = "key1".to_string();
        let key_destination: String = "key2".to_string();
        let rename = redis.execute(Command::Rename {
            key_origin,
            key_destination,
        });
        assert!(rename.is_ok());

        let key: String = "key1".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(get.is_err());

        let key: String = "key2".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(get.is_ok());
        assert_eq!("value1".to_string(), get.unwrap().to_string());
    }

    #[test]
    fn test_lindex_with_key_used_err() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "key".to_string();
        let index = 1;
        let lindex = redis.execute(Command::Lindex { key, index });

        assert!(lindex.is_err());
    }

    #[test]
    fn test_lindex_ok() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key: String = "key".to_string();
        let index = 0;
        let lindex = redis.execute(Command::Lindex { key, index });

        println!("{:?}", redis);

        assert!(lindex.is_ok());
        assert_eq!("value2".to_string(), lindex.unwrap())
    }

    #[test]
    fn test_lindex_negative_index_ok() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key: String = "key".to_string();
        let index = -1;
        let lindex = redis.execute(Command::Lindex { key, index });

        assert!(lindex.is_ok());
        assert_eq!("value".to_string(), lindex.unwrap())
    }

    #[test]
    fn test_lindex_negative_index_result_nil_ok() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key: String = "key".to_string();
        let index = -3;
        let lindex = redis.execute(Command::Lindex { key, index });

        println!("{:?}", redis);

        assert!(lindex.is_ok());
        assert_eq!("nil".to_string(), lindex.unwrap())
    }

    #[test]
    fn test_llen_key_saved_as_string_ok() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "key".to_string();
        let llen = redis.execute(Command::Llen { key });

        assert!(llen.is_err());
    }

    #[test]
    fn test_llen_key_not_found_ok() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let llen = redis.execute(Command::Llen { key });

        assert!(llen.is_ok());
        assert_eq!("0".to_string(), llen.unwrap())
    }

    #[test]
    fn test_llen_key_used_twice_ok() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key: String = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key: String = "key".to_string();
        let llen = redis.execute(Command::Llen { key });

        assert_eq!("4".to_string(), llen.unwrap())
    }

    #[test]
    fn test_lpush_ok() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpush = redis.execute(Command::Lpush { key, value });

        assert!(lpush.is_ok());
        assert_eq!("2".to_string(), lpush.unwrap())
    }

    #[test]
    fn test_lpush_with_key_used_err() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpush = redis.execute(Command::Lpush { key, value });

        assert!(lpush.is_err());
    }

    #[test]
    fn test_lpush_key_used_ok() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpush = redis.execute(Command::Lpush { key, value });

        assert!(lpush.is_ok());
        assert_eq!("2".to_string(), lpush.unwrap());

        let key: String = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpush = redis.execute(Command::Lpush { key, value });

        assert!(lpush.is_ok());
        assert_eq!("4".to_string(), lpush.unwrap())
    }

    #[test]
    fn test_lpush_key_used_check_ok() {
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let value = vec!["1".to_string(), "2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key: String = "key".to_string();
        let value = vec!["3".to_string(), "4".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key: String = "key".to_string();
        let index = -1;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert_eq!("1".to_string(), lindex.unwrap());
        let key: String = "key".to_string();
        let index = -2;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert_eq!("2".to_string(), lindex.unwrap());
        let key: String = "key".to_string();
        let index = -3;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert_eq!("3".to_string(), lindex.unwrap());
        let key: String = "key".to_string();
        let index = -4;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert_eq!("4".to_string(), lindex.unwrap());
    }

    #[test]
    fn test_sadd(){
        let mut redis: Redis = Redis::new();

        let key: String = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let sadd = redis.execute(Command::Sadd { key, values });

        assert_eq!("3".to_string(), sadd.unwrap())
    }

    #[test]
    fn test_sadd_with_existing_key(){
        let mut redis: Redis = Redis::new();

        let key: String = "set".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let sadd = redis.execute(Command::Sadd { key, values });

        assert_eq!("3".to_string(), sadd.unwrap());

        let key: String = "set".to_string();
        let mut values = HashSet::new();
        values.insert("value3".to_string());
        values.insert("value4".to_string());

        let sadd2 = redis.execute(Command::Sadd { key, values });
        assert_eq!("1".to_string(), sadd2.unwrap());
    }

    #[test]
    fn test_sadd_error(){
        let mut redis: Redis = Redis::new();

        let key: String = "set".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key: String = "set".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let sadd = redis.execute(Command::Sadd { key, values });

        assert_eq!("WRONGTYPE A hashset data type expected".to_string(), sadd.err().unwrap())
    }
}

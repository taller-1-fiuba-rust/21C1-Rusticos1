use crate::entities::redis_element::RedisElement;
use std::collections::hash_map::Keys;
use std::collections::HashMap;
use std::hash::Hash;
use std::str::from_utf8;
use std::time::{Duration, SystemTime};
use std::vec::Drain;

#[derive(Debug)]
/// Estructura para almacenar los pares clave-valor con su expiración, si corresponde. Además maneja los últimos accesos a cada clave.
pub struct TtlHashMap<K: Eq + Hash, V> {
    store: HashMap<K, V>,
    ttls: HashMap<K, SystemTime>,
    last_access: HashMap<K, SystemTime>,
}

impl<K: Clone + Eq + Hash, V> TtlHashMap<K, V> {
    /// Constructor de un nuevo TtlHashMap
    pub fn new() -> Self {
        TtlHashMap {
            store: HashMap::new(),
            ttls: HashMap::new(),
            last_access: HashMap::new(),
        }
    }

    fn expired(&self, key: &K) -> bool {
        match self.ttls.get(key) {
            Some(ttl) => ttl.elapsed().is_ok(),
            None => false,
        }
    }

    /// Actualiza el último acceso a la clave.
    /// Devuelve el tiempo transcurrido desde el anterior acceso, o None si no existe la clave o expiró.
    pub fn update_last_access(&mut self, key: &K) -> Option<Duration> {
        if !self.contains_key(key) {
            return None;
        }

        self.last_access
            .insert(key.clone(), SystemTime::now())
            .map(|value| value.elapsed().unwrap_or_else(|_| Duration::from_secs(0)))
    }

    /// Setea una expiración para la clave a partir de una Duration.
    /// Devuelve None si no existe la clave, y SystemTime::UNIX_EPOCH si era persistente. Sino, devuelve el valor previo de ttl.
    pub fn set_ttl_relative(&mut self, key: K, duration: Duration) -> Option<SystemTime> {
        if !self.contains_key(&key) {
            return None;
        }
        let ttl = SystemTime::now() + duration;
        Some(self.ttls.insert(key, ttl).unwrap_or(SystemTime::UNIX_EPOCH))
    }

    /// Setea una expiración para la clave en un cierto SystemTime.
    /// Devuelve None si no existe la clave, y SystemTime::UNIX_EPOCH si era persistente. Sino, devuelve el valor previo de ttl.
    pub fn set_ttl_absolute(&mut self, key: K, ttl: SystemTime) -> Option<SystemTime> {
        if !self.contains_key(&key) {
            return None;
        }
        Some(self.ttls.insert(key, ttl).unwrap_or(SystemTime::UNIX_EPOCH))
    }

    /// Elimina la expiración de la clave.
    /// Devuelve el ttl que tenía. Si no tenía una expiración, devuelve None.
    pub fn delete_ttl(&mut self, key: &K) -> Option<SystemTime> {
        if self.expired(key) {
            self.remove(key);
            return None;
        }
        self.ttls.remove(key)
    }

    /// Obtiene el tiempo restante de expiración de una clave como Duration.
    /// Devuelve None si no existe la clave, y una duración 0 si existe pero es persistente. Sino, devuelve el ttl.
    pub fn get_ttl(&mut self, key: &K) -> Option<Duration> {
        if !self.contains_key(key) {
            return None;
        }
        let ttl = match self.ttls.get(key) {
            Some(value) => value
                .duration_since(SystemTime::now())
                .unwrap_or_else(|_| Duration::from_secs(0)),
            None => Duration::from_secs(0),
        };
        Some(ttl)
    }

    /// Devuelve la cantidad de claves guardadas, sin chequear que no hayan expirado.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Guarda un par clave-valor.
    pub fn insert(&mut self, key: K, value: V) {
        self.remove(&key);
        self.last_access.insert(key.clone(), SystemTime::now());
        self.store.insert(key, value);
    }

    /// Devuelve si una clave existe o no, chequeando que no haya expirado. Si expiró, la borra.
    pub fn contains_key(&mut self, key: &K) -> bool {
        match self.store.get(key) {
            Some(_value) => {
                if self.expired(key) {
                    self.remove(key);
                    return false;
                }
                true
            }
            None => false,
        }
    }

    /// Elimina el par clave-valor.
    /// Devuelve el valor, y si no existía la clave, devuelve None.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.ttls.remove(key);
        self.last_access.remove(key);
        self.store.remove(key)
    }

    /// Devuelve el valor correspondiente a la clave. Si expiró, la borra y devuelve None.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.expired(key) {
            self.remove(key);
            return None;
        }
        self.update_last_access(key);
        self.store.get(key)
    }

    /// Devuelve una referencia mutable del valor correspondiente a la clave. Si expiró, la borra y devuelve None.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if self.expired(key) {
            self.remove(key);
            return None;
        }
        self.update_last_access(key);
        self.store.get_mut(key)
    }

    /// Devuelve todas las claves, sin chequear que no hayan expirado.
    pub fn keys(&self) -> Keys<K, V> {
        self.store.keys()
    }

    fn set_size(&mut self, store_size: usize, ttl_size: usize) {
        self.store.reserve(store_size);
        self.ttls.reserve(ttl_size);
    }
}

const OP_EOF: u8 = 0xff;
const OP_EXPIRETIME: u8 = 0xfd;
const OP_RESIZEDB: u8 = 0xfb;
const WRONG_ELEMENT_TYPE: u8 = 3;

impl TtlHashMap<String, RedisElement> {
    /// Devuelve un vector de bytes con el TtlHashMap serializado según el estandar de REDIS.
    pub fn serialize(&self) -> Vec<u8> {
        let mut s: Vec<u8> = vec![OP_RESIZEDB];
        s.append(&mut TtlHashMap::length_encode(self.store.len()));
        s.append(&mut TtlHashMap::length_encode(self.ttls.len()));

        for (key, value) in self.store.iter() {
            if let Some(ttl) = self.ttls.get(key) {
                let secs = ttl
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_else(|_| Duration::from_secs(0))
                    .as_secs();
                s.push(OP_EXPIRETIME);
                s.append(&mut (secs as u32).to_be_bytes().to_vec());
            }
            let value_type = TtlHashMap::value_type_encode(value);
            if value_type != WRONG_ELEMENT_TYPE {
                s.push(value_type);
                s.append(&mut TtlHashMap::string_encode(key.to_string()));
                s.append(&mut TtlHashMap::value_encode(value.clone()));
            }
        }
        s.push(OP_EOF);
        s
    }

    // Deserializa un vector de bytes para devolver un TtlHashMap cargado con todos los RedisElements.
    pub fn deserialize(mut s: Vec<u8>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut map: TtlHashMap<String, RedisElement> = TtlHashMap::new();
        let mut s = s.drain(..);

        match s.next().unwrap_or(0) {
            OP_RESIZEDB => {
                map.set_size(
                    TtlHashMap::length_decode(&mut s).ok_or("Corrupt store size")? as usize,
                    TtlHashMap::length_decode(&mut s).ok_or("Corrupt ttl size")? as usize,
                );
                map.load(&mut s)?;
                Ok(map)
            }
            OP_EOF => Ok(map),
            _ => Err("Found unknown OP code.".into()),
        }
    }

    fn load(&mut self, s: &mut Drain<'_, u8>) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(op_code) = s.next() {
            match op_code {
                OP_EXPIRETIME => {
                    let secs = TtlHashMap::read_int(s).ok_or("Corrupt expiry time")?;
                    let ttl = SystemTime::UNIX_EPOCH + Duration::from_secs(secs as u64);

                    if SystemTime::now().duration_since(ttl).is_err() {
                        let value_type = s.next().ok_or("Corrupt value type")?;
                        let key = TtlHashMap::string_decode(s).ok_or("Corrupt key")?;
                        let value =
                            TtlHashMap::value_decode(s, value_type).ok_or("Corrupt value")?;
                        self.insert(key.clone(), value);
                        self.set_ttl_absolute(key, ttl);
                    }
                }
                OP_EOF => (),
                _ => {
                    let value_type = op_code;
                    let key = TtlHashMap::string_decode(s).ok_or("Corrupt key")?;
                    let value = TtlHashMap::value_decode(s, value_type).ok_or("Corrupt value")?;
                    self.insert(key, value);
                }
            }
        }
        Ok(())
    }

    fn bytes_as_u32_be(bytes: &[u8]) -> u32 {
        ((bytes[0] as u32) << 24)
            | ((bytes[1] as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | (bytes[3] as u32)
    }

    fn read_int(s: &mut Drain<'_, u8>) -> Option<u32> {
        Some(TtlHashMap::bytes_as_u32_be(&[
            s.next()?,
            s.next()?,
            s.next()?,
            s.next()?,
        ]))
    }

    fn string_decode(s: &mut Drain<'_, u8>) -> Option<String> {
        let mut bytes: Vec<u8> = Vec::new();
        let len = TtlHashMap::length_decode(s)?;
        for _ in 0..len {
            bytes.push(s.next()?);
        }
        Some(from_utf8(&bytes).ok()?.to_string())
    }

    pub fn string_encode(string: String) -> Vec<u8> {
        let mut bytes: Vec<u8> = vec![];
        bytes.append(&mut TtlHashMap::length_encode(string.len()));
        bytes.append(&mut string.as_bytes().to_vec());
        bytes
    }

    pub fn list_encode(list: Vec<String>) -> Vec<u8> {
        let mut bytes = TtlHashMap::length_encode(list.len());
        for value in list {
            bytes.append(&mut TtlHashMap::string_encode(value));
        }
        bytes
    }

    fn list_decode(s: &mut Drain<'_, u8>) -> Option<Vec<String>> {
        let len = TtlHashMap::length_decode(s)?;
        let mut vec: Vec<String> = vec![];
        for _ in 0..len {
            vec.push(TtlHashMap::string_decode(s)?);
        }
        Some(vec)
    }

    pub fn length_encode(length: usize) -> Vec<u8> {
        if length < 64 {
            // 00 + length in 6 bits
            vec![length as u8]
        } else if length < 16384 {
            // 01 + length in 14 bits
            vec![0x40 | (length >> 8) as u8, length as u8]
        } else {
            // 1000 0000 + length in 32 bits
            vec![
                0x80,
                (length >> 24) as u8,
                (length >> 16) as u8,
                (length >> 8) as u8,
                length as u8,
            ]
        }
    }

    fn length_decode(s: &mut Drain<'_, u8>) -> Option<u32> {
        let first_byte = s.next().unwrap_or(5);
        match first_byte >> 6 {
            0b00 => Some(first_byte as u32),
            0b01 => Some(TtlHashMap::bytes_as_u32_be(&[
                0,
                0,
                first_byte & 0b00111111,
                s.next()?,
            ])),
            0b10 => Some(TtlHashMap::read_int(s)?),
            _ => None,
        }
    }

    pub fn value_encode(value: RedisElement) -> Vec<u8> {
        match value {
            RedisElement::String(string) => TtlHashMap::string_encode(string),
            RedisElement::List(list) => TtlHashMap::list_encode(list),
            RedisElement::Set(set) => TtlHashMap::list_encode(set.into_iter().collect()),
            _ => vec![],
        }
    }

    fn value_decode(s: &mut Drain<'_, u8>, value_type: u8) -> Option<RedisElement> {
        match value_type {
            0 => Some(RedisElement::String(TtlHashMap::string_decode(s)?)),
            1 => Some(RedisElement::List(TtlHashMap::list_decode(s)?)),
            2 => Some(RedisElement::Set(
                TtlHashMap::list_decode(s)?.into_iter().collect(),
            )),
            _ => None,
        }
    }

    pub fn value_type_encode(value: &RedisElement) -> u8 {
        match value {
            RedisElement::String(_) => 0,
            RedisElement::List(_) => 1,
            RedisElement::Set(_) => 2,
            _ => WRONG_ELEMENT_TYPE,
        }
    }
}

#[allow(unused_imports)]
mod test {
    use crate::entities::ttl_hash_map::RedisElement;
    use crate::entities::ttl_hash_map::TtlHashMap;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_get_key() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        map.insert(key.clone(), 1);

        assert_eq!(map.get(&key).unwrap(), &1);
    }

    #[test]
    fn test_get_mut_key_modifies_key() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        map.insert(key.clone(), 1);
        let value = map.get_mut(&key).unwrap();
        *value += 1;

        assert_eq!(map.get(&key).unwrap(), &2);
    }

    #[ignore]
    #[test]
    fn test_ttl_relative_deletes_key() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        map.insert(key.clone(), 1);
        map.set_ttl_relative(key.clone(), Duration::from_secs(1));

        std::thread::sleep(Duration::from_secs(1));

        assert!(map.get(&key).is_none());
    }

    #[test]
    fn test_ttl_absolute_deletes_key() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        map.insert(key.clone(), 1);
        map.set_ttl_absolute(key.clone(), SystemTime::now());

        assert!(map.get(&key).is_none());
    }

    #[test]
    fn test_contains_key_on_expired_key() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        map.insert(key.clone(), 1);
        map.set_ttl_absolute(key.clone(), SystemTime::now());

        assert!(!map.contains_key(&key));
    }

    #[test]
    fn test_delete_ttl_on_expired_key() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        map.insert(key.clone(), 1);
        map.set_ttl_absolute(key.clone(), SystemTime::now());

        assert!(map.delete_ttl(&key).is_none());
        assert!(map.get(&key).is_none());
    }

    #[test]
    fn test_delete_ttl_on_presistent_key() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        map.insert(key.clone(), 1);

        assert!(map.delete_ttl(&key).is_none());
        assert!(map.get(&key).is_some());
    }

    #[test]
    fn test_insert_key_twice_deletes_ttl() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        map.insert(key.clone(), 1);
        map.set_ttl_absolute(key.clone(), SystemTime::now());
        map.insert(key.clone(), 1);

        assert!(map.delete_ttl(&key).is_none());
        assert!(map.get(&key).is_some());
    }

    #[ignore]
    #[test]
    fn test_new_key_last_access() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        map.insert(key.clone(), 1);
        std::thread::sleep(Duration::from_secs(1));

        assert_eq!(map.update_last_access(&key).unwrap().as_secs(), 1);
    }

    #[ignore]
    #[test]
    fn test_get_changes_last_access() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        map.insert(key.clone(), 1);
        std::thread::sleep(Duration::from_secs(1));
        map.get(&key);

        assert_eq!(map.update_last_access(&key).unwrap().as_secs(), 0);
    }

    #[test]
    fn test_remove_key_and_add_again() {
        let mut map: TtlHashMap<String, u8> = TtlHashMap::new();
        let key = "key".to_string();

        assert!(map.remove(&key).is_none());
    }

    #[test]
    fn test_length_encode_decode() {
        let number: u32 = 15;
        let mut encoded: Vec<u8> = TtlHashMap::length_encode(number as usize);
        let decoded: u32 = TtlHashMap::length_decode(&mut encoded.drain(..)).unwrap();
        assert_eq!(number, decoded);
    }

    #[test]
    fn test_value_encode_decode() {
        let value = RedisElement::String("value".to_string());
        let mut encoded = TtlHashMap::value_encode(value.clone());
        let decoded = TtlHashMap::value_decode(
            &mut encoded.drain(..),
            TtlHashMap::value_type_encode(&value),
        );
        assert_eq!(value, decoded.unwrap());
    }

    #[test]
    fn test_serialize() {
        let mut map: TtlHashMap<String, RedisElement> = TtlHashMap::new();
        let key = "key".to_string();
        let value = RedisElement::String("value".to_string());
        map.insert(key.clone(), value.clone());
        map.set_ttl_relative(key.clone(), Duration::from_secs(2));
        let bytes = map.serialize();

        let op_resizedb = 0xfb;
        let mut store_len = TtlHashMap::length_encode(1);
        let mut ttl_len = TtlHashMap::length_encode(1);
        let op_expiretime = 0xfd;
        let secs = (SystemTime::now() + Duration::from_secs(2))
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut ttl = (secs as u32).to_be_bytes().to_vec();
        let byte_value_type = TtlHashMap::value_type_encode(&RedisElement::String("".to_string()));
        let mut key_encoded = TtlHashMap::string_encode(key);
        let mut value_encoded = TtlHashMap::string_encode("value".to_string());
        let op_eof = 0xff;

        let mut vec = vec![op_resizedb];
        vec.append(&mut store_len);
        vec.append(&mut ttl_len);
        vec.push(op_expiretime);
        vec.append(&mut ttl);
        vec.push(byte_value_type);
        vec.append(&mut key_encoded);
        vec.append(&mut value_encoded);
        vec.push(op_eof);
        assert_eq!(bytes, vec);
    }

    #[test]
    fn test_deserialize() {
        let op_resizedb = 0xfb;
        let mut store_len = TtlHashMap::length_encode(1);
        let mut ttl_len = TtlHashMap::length_encode(0);
        let byte_value_type = TtlHashMap::value_type_encode(&RedisElement::String("".to_string()));
        let key = "key".to_string();
        let mut key_encoded = TtlHashMap::string_encode(key.clone());
        let mut value_encoded = TtlHashMap::string_encode("value".to_string());
        let op_eof = 0xff;

        let mut bytes = vec![op_resizedb];
        bytes.append(&mut store_len);
        bytes.append(&mut ttl_len);
        bytes.push(byte_value_type);
        bytes.append(&mut key_encoded);
        bytes.append(&mut value_encoded);
        bytes.push(op_eof);

        let mut map = TtlHashMap::deserialize(bytes).unwrap();

        assert_eq!(
            *map.get(&key).unwrap(),
            RedisElement::String("value".to_string())
        );
    }

    #[test]
    fn test_serialize_and_deserialize_key_value_string() {
        let mut map: TtlHashMap<String, RedisElement> = TtlHashMap::new();
        let key = "key".to_string();
        let value = RedisElement::String("value".to_string());
        map.insert(key.clone(), value.clone());

        let bytes = map.serialize();
        let mut new_map = TtlHashMap::deserialize(bytes).unwrap();

        assert_eq!(*new_map.get(&key).unwrap(), value);
    }

    #[test]
    fn test_serialize_and_deserialize_key_value_list() {
        let mut map: TtlHashMap<String, RedisElement> = TtlHashMap::new();
        let key = "key".to_string();
        let value = RedisElement::List(vec!["1".to_string(), "2".to_string()]);
        map.insert(key.clone(), value.clone());

        let bytes = map.serialize();
        let mut new_map = TtlHashMap::deserialize(bytes).unwrap();

        assert_eq!(*new_map.get(&key).unwrap(), value);
    }

    #[test]
    fn test_serialize_and_deserialize_with_ttl() {
        let mut map: TtlHashMap<String, RedisElement> = TtlHashMap::new();
        let key = "key".to_string();
        let ttl = SystemTime::now() + Duration::from_secs(2);

        map.insert(key.clone(), RedisElement::String("value".to_string()));
        map.set_ttl_absolute(key.clone(), ttl);

        let bytes = map.serialize();

        let mut new_map = TtlHashMap::deserialize(bytes).unwrap();
        assert_eq!(new_map.get(&key).unwrap().to_string(), "value");
        assert_eq!(
            new_map.get_ttl(&key).unwrap().as_secs(),
            ttl.duration_since(SystemTime::now()).unwrap().as_secs()
        );
    }
}

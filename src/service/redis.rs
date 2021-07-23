use crate::entities::command::Command;
use crate::entities::info_param::InfoParam;
use crate::entities::log::Log;
use crate::entities::log_level::LogLevel;
use crate::entities::pubsub_param::PubSubParam;
use crate::entities::redis_element::{RedisElement as Re, RedisElement};
use crate::entities::response::Response;
use crate::entities::ttl_hash_map::TtlHashMap;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::io::Write;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, SystemTime};
use std::{fs, process};

const WRONGTYPE_MSG: &str = "WRONGTYPE Operation against a key holding the wrong kind of value";
const OUT_OF_RANGE_MSG: &str = "ERR value is not an integer or out of range";

#[derive(Debug)]
pub struct Redis {
    db: TtlHashMap<String, RedisElement>,
    log_sender: Sender<Log>,
    vec_senders: Vec<Sender<Re>>,
    //pubsub
    subscribers: HashMap<String, Vec<Sender<Re>>>,
    users_connected: u64,
    server_time: SystemTime,
}

impl Redis {
    #[allow(dead_code)]
    pub fn new(log_sender: Sender<Log>) -> Self {
        let db = TtlHashMap::new();
        let vec_senders: Vec<Sender<Re>> = Vec::new();

        Self {
            db,
            log_sender,
            vec_senders,
            users_connected: 0,
            subscribers: HashMap::new(),
            server_time: SystemTime::now(),
        }
    }

    #[allow(dead_code)]
    fn new_for_test() -> Self {
        let db = TtlHashMap::new();
        let (log_sender, _): (Sender<Log>, _) = mpsc::channel();
        let vec_senders: Vec<Sender<Re>> = Vec::new();

        Self {
            db,
            log_sender,
            vec_senders,
            users_connected: 0,
            subscribers: HashMap::new(),
            server_time: SystemTime::now(),
        }
    }

    #[allow(dead_code)]
    pub fn execute(&mut self, command: Command) -> Result<Response, String> {
        self.notify_monitor(&command);

        match command {
            // Server
            Command::Ping => self.ping_method(),
            Command::Flushdb => Ok(self.flushdb_method()),
            Command::Dbsize => Ok(self.dbsize_method()),
            Command::Monitor => self.monitor_method(),
            Command::Info { param } => self.info_method(param),

            // System
            Command::Store { path } => self.store_method(path),
            Command::Load { path } => self.load_method(path),
            Command::AddClient => Ok(self.addclient_method()),
            Command::RemoveClient => Ok(self.removeclient_method()),

            // Strings
            Command::Append { key, value } => self.append_method(key, value),
            Command::Decrby { key, decrement } => self.incrby_method(key, -(decrement as i32)),
            Command::Get { key } => match self.get_method(key) {
                Ok(re) => Ok(Response::Normal(re)),
                Err(e) => Err(e),
            },
            Command::Getdel { key } => match self.getdel_method(key) {
                Ok(re) => Ok(Response::Normal(re)),
                Err(e) => Err(e),
            },
            Command::Getset { key, value } => self.getset_method(key, value),
            Command::Incrby { key, increment } => self.incrby_method(key, increment as i32),
            Command::Mget { keys } => Ok(self.mget_method(keys)),
            Command::Mset { key_values } => Ok(self.mset_method(key_values)),
            Command::Set { key, value } => {
                Ok(Response::Normal(Re::String(self.set_method(key, value))))
            }
            Command::Strlen { key } => self.strlen_method(key),

            // Keys
            Command::Copy {
                key_origin,
                key_destination,
            } => Ok(self.copy_method(key_origin, key_destination)),
            Command::Del { keys } => Ok(Response::Normal(Re::String(self.del_method(keys)))),
            Command::Exists { keys } => Ok(self.exists_method(keys)),
            Command::Expire { key, ttl } => {
                Ok(Response::Normal(Re::String(self.expire_method(key, ttl))))
            }
            Command::Expireat { key, ttl } => {
                Ok(Response::Normal(Re::String(self.expireat_method(key, ttl))))
            }
            Command::Persist { key } => Ok(Response::Normal(Re::String(self.persist_method(key)))),
            Command::Rename {
                key_origin,
                key_destination,
            } => self.rename_method(key_origin, key_destination),
            Command::Keys { pattern } => Ok(Response::Normal(Re::List(self.keys_method(pattern)))),
            Command::Touch { keys } => Ok(Response::Normal(Re::String(self.touch_method(keys)))),
            Command::Ttl { key } => Ok(Response::Normal(Re::String(self.ttl_method(key)))),
            Command::Type { key } => Ok(Response::Normal(Re::String(self.type_method(key)))),

            // Lists
            Command::Lindex { key, index } => self.lindex_method(key, index),
            Command::Llen { key } => self.llen_method(key),
            Command::Lpop { key, count } => self.lpop_method(key, count),
            Command::Lpush { key, value } => self.lpush_method(key, value),
            Command::Lpushx { key, value } => self.lpushx_method(key, value),
            Command::Lrange { key, begin, end } => self.lrange_method(key, begin, end),
            Command::Lrem {
                key,
                count,
                element,
            } => self.lrem_method(key, count, element),
            Command::Lset {
                key,
                index,
                element,
            } => self.lset_method(key, index, element),
            Command::Rpop { key, count } => self.rpop_method(key, count),
            Command::Rpush { key, value } => self.rpush_method(key, value),
            Command::Rpushx { key, value } => self.rpushx_method(key, value),

            //Sets
            Command::Sadd { key, values } => self.sadd_method(key, values),
            Command::Scard { key } => self.scard_method(key),
            Command::Sismember { key, value } => self.sismember_method(key, value),
            Command::Smembers { key } => self.smembers_method(key),
            Command::Srem { key, values } => self.srem_method(key, values),

            // Pubsub
            Command::Pubsub { param } => self.pubsub_method(param),
            Command::Subscribe { channels } => self.subscribe_method(channels),
            Command::Publish { channel, message } => self.publish_method(channel, message),
            Command::Unsubscribe { channels } => Err("Method not implemented".to_string()),
        }
    }

    fn pubsub_method(&mut self, param: PubSubParam) -> Result<Response, String> {
        match param {
            PubSubParam::Channels => self.channels_method(),
            PubSubParam::ChannelsWithChannel(channel) => self.channels_with_channel_method(channel),
            PubSubParam::Numsub => self.numsub_method(),
            PubSubParam::NumsubWithChannels(channels) => self.numsub_with_channels_method(channels),
        }
    }

    fn channels_method(&mut self) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command Pubsub Channels Received".to_string(),
        ));

        let mut vec_response = vec![];
        for (key, _) in self.subscribers.iter() {
            vec_response.push(key.to_string());
        }

        Ok(Response::Normal(Re::List(vec_response)))
    }

    fn channels_with_channel_method(&mut self, channel: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command Pubsub Channels Received".to_string(),
        ));

        let mut vec_response = vec![];
        for (key, _) in self.subscribers.iter() {
            if channel == key.to_string() {
                vec_response.push(key.to_string());
            }
        }

        Ok(Response::Normal(Re::List(vec_response)))
    }

    fn numsub_method(&mut self) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command Pubsub Numsub Received".to_string(),
        ));

        Ok(Response::Normal(Re::List(vec![])))
    }

    fn numsub_with_channels_method(&mut self, channels: Vec<String>) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command Pubsub Numsub Received".to_string(),
        ));

        let mut vec_response = vec![];
        for (key, value) in self.subscribers.iter() {
            if channels.iter().any(|i| i.to_string() == key.to_string()) {
                vec_response.push(key.to_string());
                vec_response.push(value.len().to_string());
            } else {
                vec_response.push(key.to_string());
                vec_response.push("0".to_string());
            }
        }

        Ok(Response::Normal(Re::List(vec_response)))
    }

    fn subscribe_method(&mut self, channels: Vec<String>) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command Subscribe Received".to_string(),
        ));

        let (sen, rec): (Sender<Re>, Receiver<Re>) = mpsc::channel();
        for channel in channels {
            let mut vector_sender;

            if let Some(vector) = self.subscribers.get_mut(&channel) {
                vector_sender = vector.clone();
                vector_sender.push(sen.clone());
            } else {
                vector_sender = vec![sen.clone()];
            }

            self.subscribers
                .insert(channel.clone(), vector_sender.to_vec());
            // TODO: Revisar que hacer con este
            let _result = sen.clone().send(Re::List(vec![
                "subscribe".to_string(),
                channel,
                "1".to_string(),
            ]));
        }
        return Ok(Response::Stream(rec));
    }

    fn publish_method(&mut self, channel: String, msg: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command Publish Received".to_string(),
        ));

        if !self.subscribers.contains_key(&channel) {
            return Ok(Response::Normal(Re::String("0".to_string())));
        }

        if let Some(vector) = self.subscribers.get_mut(&channel) {
            for x in vector {
                // TODO: revisar salida
                let _ = x.send(Re::String(msg.to_string()));
            }
        }

        Ok(Response::Normal(Re::String("Ok".to_string())))
    }

    fn addclient_method(&mut self) -> Response {
        self.users_connected += 1;
        Response::Normal(RedisElement::String("Ok".to_string()))
    }

    fn removeclient_method(&mut self) -> Response {
        self.users_connected -= 1;
        Response::Normal(RedisElement::String("Ok".to_string()))
    }

    fn info_method(&mut self, param: InfoParam) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command Info Received".to_string(),
        ));

        match param {
            InfoParam::ConnectedClients => Ok(Response::Normal(RedisElement::String(
                self.users_connected.to_string(),
            ))),
            InfoParam::Port => Err("Not Implemented".to_string()),
            InfoParam::ConfigFile => Err("Not Implemented".to_string()),
            InfoParam::Uptime => self.get_server_uptime(),
            InfoParam::ServerTime => Err("Not Implemented".to_string()),
            InfoParam::ProcessID => Ok(Response::Normal(RedisElement::String(
                process::id().to_string(),
            ))),
            _ => Err("Not Implemented".to_string()),
        }
    }

    fn get_server_uptime(&mut self) -> Result<Response, String> {
        let result_time = SystemTime::now().duration_since(self.server_time);
        match result_time {
            Ok(duration) => Ok(Response::Normal(RedisElement::String(
                duration.as_secs().to_string(),
            ))),
            Err(e) => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    e.to_string(),
                ));
                Err(e.to_string())
            }
        }
    }

    fn dbsize_method(&mut self) -> Response {
        Response::Normal(Re::String(self.db.len().to_string()))
    }

    fn ping_method(&mut self) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command PING Received".to_string(),
        ));

        Ok(Response::Normal(Re::String("PONG".to_string())))
    }

    fn notify_monitor(&mut self, command: &Command) {
        /*
            let empty_vec = vec<sender>
            for sender in vec_senders {
                match let result = sender.send(command.to_string()) {
                    Ok(_) => empty_vec.push(sender),
                    Err(_) => _,
                }
            }
            Similar al flag del otro.
        */

        for sender in &self.vec_senders {
            // TODO: Revisar que hacer con este porque queda viviendo y el send no muere
            let command_str = command.as_str().to_string();
            if command_str != "" {
                let _result = sender.send(Re::String(command_str));
            }
        }
    }

    fn monitor_method(&mut self) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command MONITOR Received".to_string(),
        ));

        let (sen, rec): (Sender<Re>, Receiver<Re>) = mpsc::channel();

        let sen_clone = sen.clone();
        // TODO: Revisar que hacer con este
        let result = sen_clone.send(Re::String("OK".to_string()));
        match result {
            Ok(_) => {
                self.vec_senders.push(sen);

                return Ok(Response::Stream(rec));
            }
            Err(e) => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    e.to_string(),
                ));
                Err("Error processing Monitor Method".to_string())
            }
        }
    }

    fn flushdb_method(&mut self) -> Response {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command FLUSHDB Received".to_string(),
        ));

        self.db = TtlHashMap::new();
        Response::Normal(Re::String("OK".to_string()))
    }

    #[allow(dead_code)]
    fn copy_method(&mut self, key_origin: String, key_destination: String) -> Response {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command COPY Received - key origin:".to_string()
                + &*key_origin
                + " - key destination: "
                + &*key_destination,
        ));

        let value_origin;
        match self.db.get(&key_origin) {
            Some(value) => value_origin = value.clone(),
            None => return Response::Normal(Re::String("0".to_string())),
        };

        match self.db.get(&key_destination) {
            Some(_) => Response::Normal(Re::String("0".to_string())),
            None => {
                self.db.insert(key_destination, value_origin);
                Response::Normal(Re::String("1".to_string()))
            }
        }
    }

    #[allow(dead_code)]
    fn get_method(&mut self, key: String) -> Result<Re, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command GET Received - key: ".to_string() + &*key,
        ));

        match self.db.get(&key) {
            Some(return_value) => match return_value {
                Re::String(s) => Ok(Re::String(s.to_string())),
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => Ok(Re::Nil),
        }
    }

    fn strlen_method(&mut self, key: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command STRLEN Received - key: ".to_string() + &*key,
        ));

        match self.db.get(&key) {
            Some(return_value) => match return_value {
                Re::String(s) => Ok(Response::Normal(Re::String(s.len().to_string()))),
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => Ok(Response::Normal(Re::String("0".to_string()))),
        }
    }

    #[allow(dead_code)]
    fn getset_method(&mut self, key: String, value: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command GETSET Received - key: ".to_string() + &*key,
        ));

        match self.get_method(key.clone()) {
            Ok(return_value) => {
                self.set_method(key, value);
                Ok(Response::Normal(return_value))
            }
            Err(e) => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    e.clone(),
                ));
                Err(e)
            }
        }
    }

    #[allow(dead_code)]
    fn set_method(&mut self, key: String, value: String) -> String {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command SET Received - key: ".to_string() + &*key,
        ));

        self.db.insert(key, Re::String(value));

        "Ok".to_string()
    }

    #[allow(dead_code)]
    fn incrby_method(&mut self, key: String, increment: i32) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command INCRBY Received - key: ".to_string() + &*key,
        ));

        match self.get_method(key.clone()) {
            Ok(return_value) => match return_value {
                Re::String(value) => {
                    let my_int: Result<i32, _> = value.parse();
                    if my_int.is_err() {
                        let _ = self.log_sender.send(Log::new(
                            LogLevel::Error,
                            line!(),
                            column!(),
                            file!().to_string(),
                            OUT_OF_RANGE_MSG.to_string(),
                        ));

                        return Err(OUT_OF_RANGE_MSG.to_string());
                    }

                    let my_int = my_int.unwrap() + increment;
                    Ok(Response::Normal(Re::String(
                        self.set_method(key, my_int.to_string()),
                    )))
                }
                Re::Nil => Ok(Response::Normal(Re::String(
                    self.set_method(key, increment.to_string()),
                ))),
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            Err(_) => Ok(Response::Normal(Re::String(
                self.set_method(key, increment.to_string()),
            ))),
        }
    }

    #[allow(dead_code)]
    fn mget_method(&mut self, keys: Vec<String>) -> Response {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command MGET Received - keys: ".to_string() + &keys.join(" - "),
        ));

        let mut elements: Vec<String> = Vec::new();
        for key in keys.iter() {
            elements.push(
                self.get_method(key.to_string())
                    .unwrap_or(Re::Nil)
                    .to_string(),
            );
        }
        Response::Normal(Re::List(elements))
    }

    #[allow(dead_code)]
    fn mset_method(&mut self, key_values: Vec<(String, String)>) -> Response {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command MSET Received".to_string(),
        ));

        for (key, value) in key_values.iter() {
            self.set_method(key.to_string(), value.to_string());
        }

        Response::Normal(Re::String("Ok".to_string()))
    }

    #[allow(dead_code)]
    fn getdel_method(&mut self, key: String) -> Result<Re, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command GETDEL Received - keys: ".to_string() + &key,
        ));

        match self.get_method(key.clone()) {
            Ok(return_value) => match return_value {
                Re::String(_) => {
                    self.db.remove(&key);
                    Ok(return_value)
                }
                Re::Nil => Ok(return_value),
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            Err(msg) => Err(msg),
        }
    }

    #[allow(dead_code)]
    fn del_method(&mut self, keys: Vec<String>) -> String {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command DEL Received - keys: ".to_string() + &keys.join(" - "),
        ));

        let mut count = 0;
        for key in keys.iter() {
            if self.db.remove(&key).is_some() {
                count += 1;
            }
        }

        count.to_string()
    }

    #[allow(dead_code)]
    fn append_method(&mut self, key: String, value: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command APPEND Received - key: ".to_string() + &*key,
        ));

        match self.get_method(key.clone()) {
            Ok(redis_element) => match redis_element {
                Re::String(s) => {
                    let value = s + &value;
                    Ok(Response::Normal(Re::String(self.set_method(key, value))))
                }
                Re::Nil => Ok(Response::Normal(Re::String(self.set_method(key, value)))),
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            Err(e) => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    e.to_string(),
                ));
                Err(e)
            }
        }
    }

    fn exists_method(&mut self, keys: Vec<String>) -> Response {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command EXISTS Received - key: ".to_string() + &keys.join(" - "),
        ));

        let mut count = 0;
        for key in keys.iter() {
            if self.db.contains_key(&key) {
                count += 1;
            }
        }

        Response::Normal(Re::String(count.to_string()))
    }

    fn expire_method(&mut self, key: String, ttl: Duration) -> String {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command EXPIRE Received - key: ".to_string() + &*key,
        ));

        match self.db.set_ttl_relative(key, ttl) {
            Some(_) => "1".to_string(),
            None => "0".to_string(),
        }
    }

    fn expireat_method(&mut self, key: String, ttl: SystemTime) -> String {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command EXPIREAT Received - key: ".to_string() + &*key,
        ));

        match self.db.set_ttl_absolute(key, ttl) {
            Some(_) => "1".to_string(),
            None => "0".to_string(),
        }
    }

    fn persist_method(&mut self, key: String) -> String {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command PERSIST Received - key: ".to_string() + &*key,
        ));

        match self.db.delete_ttl(&key) {
            Some(_) => "1".to_string(),
            None => "0".to_string(),
        }
    }

    fn rename_method(
        &mut self,
        key_origin: String,
        key_destination: String,
    ) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command RENAME Received - key origin: ".to_string()
                + &*key_origin
                + " - key destination: "
                + &*key_destination,
        ));

        match self.getdel_method(key_origin) {
            Ok(value) => Ok(Response::Normal(Re::String(
                self.set_method(key_destination, value.to_string()),
            ))),
            Err(msg) => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    msg.clone(),
                ));
                Err(msg)
            }
        }
    }

    fn touch_method(&mut self, keys: Vec<String>) -> String {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command TOUCH Received - keys: ".to_string() + &keys.join(" - "),
        ));

        let mut count = 0;
        for key in keys.iter() {
            match self.db.update_last_access(&key.to_string()) {
                None => (),
                Some(time) => {
                    count += 1;
                    let time = time.as_secs().to_string();
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Debug,
                        line!(),
                        column!(),
                        file!().to_string(),
                        format!("Key {} previous access: {} secs ago.", &key, &time),
                    ));
                }
            }
        }

        count.to_string()
    }

    fn ttl_method(&mut self, key: String) -> String {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command TTL Received - key: ".to_string() + &*key,
        ));

        match self.db.get_ttl(&key) {
            Some(value) => {
                if value == Duration::from_secs(0) {
                    return "-1".to_string();
                }
                value.as_secs().to_string()
            }
            None => "-2".to_string(),
        }
    }

    fn type_method(&mut self, key: String) -> String {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command TYPE Received - key: ".to_string() + &*key,
        ));

        match self.db.get(&key) {
            Some(return_value) => match return_value {
                Re::String(_) => "string".to_string(),
                Re::List(_) => "list".to_string(),
                Re::Set(_) => "set".to_string(),
                Re::Nil => "none".to_string(),
            },
            None => "none".to_string(),
        }
    }

    fn lindex_method(&mut self, key: String, index: i32) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command LINDEX Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                Re::List(value) => {
                    let len_value = value.len() as i32;
                    let mut position: i32 = index;

                    if index < 0 {
                        position = index + len_value;
                    }

                    match value.get(position as usize) {
                        Some(saved_value) => {
                            Ok(Response::Normal(Re::String(saved_value.to_string())))
                        }
                        None => Ok(Response::Normal(Re::Nil)),
                    }
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => Ok(Response::Normal(Re::Nil)),
        }
    }

    fn llen_method(&mut self, key: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command LLEN Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                Re::List(value) => Ok(Response::Normal(Re::String(value.len().to_string()))),
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => Ok(Response::Normal(Re::String("0".to_string()))),
        }
    }

    fn lpop_method(&mut self, key: String, count: usize) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command LPOP Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                Re::List(value) => {
                    let return_value: Vec<String>;
                    let vector_to_save: Vec<String>;
                    if count == 0 {
                        return_value = Vec::from(value.get(..=count).unwrap());
                        vector_to_save = Vec::from(value.get(count + 1..).unwrap());
                    } else {
                        let qty = match count {
                            x if x >= value.len() => value.len(),
                            _ => count,
                        };
                        return_value = Vec::from(value.get(..qty).unwrap());
                        vector_to_save = Vec::from(value.get(qty..).unwrap());
                    }

                    self.db.insert(key, Re::List(vector_to_save));

                    if return_value.len() == 1 && return_value.first().is_some() {
                        let value = return_value.first();
                        return Ok(Response::Normal(Re::String(value.unwrap().to_string())));
                    }

                    match return_value.len() {
                        x if x > 0 => Ok(Response::Normal(Re::List(return_value.to_vec()))),
                        _ => Ok(Response::Normal(Re::Nil)),
                    }
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => Ok(Response::Normal(Re::Nil)),
        }
    }

    fn lpush_method(&mut self, key: String, values: Vec<String>) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command LPUSH Received - key: ".to_string() + &*key,
        ));

        let mut redis_element: Vec<String> = values;
        redis_element.reverse();

        match self.db.get_mut(&key) {
            Some(value) => match value {
                Re::List(value) => {
                    let saved_vector = value.clone();
                    redis_element.extend(saved_vector);
                    self.db.insert(key, Re::List(redis_element.clone()));

                    Ok(Response::Normal(Re::String(
                        redis_element.len().to_string(),
                    )))
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Debug,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => {
                self.db.insert(key, Re::List(redis_element.clone()));

                Ok(Response::Normal(Re::String(
                    redis_element.len().to_string(),
                )))
            }
        }
    }

    fn lpushx_method(&mut self, key: String, values: Vec<String>) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command LPUSHX Received - key: ".to_string() + &*key,
        ));

        let mut redis_element: Vec<String> = values;
        redis_element.reverse();

        match self.db.get_mut(&key) {
            Some(value) => match value {
                RedisElement::List(value) => {
                    let saved_vector = value.clone();
                    redis_element.extend(saved_vector);
                    self.db
                        .insert(key, RedisElement::List(redis_element.clone()));

                    Ok(Response::Normal(Re::String(
                        redis_element.len().to_string(),
                    )))
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => {
                self.db.insert(key, Re::List(vec![]));
                Ok(Response::Normal(Re::String("0".to_string())))
            }
        }
    }

    fn lrange_method(&mut self, key: String, begin: i32, end: i32) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command LRANGE Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                Re::List(value) => {
                    let len_value = value.len() as i32;
                    let mut begin_position: i32 = begin;

                    if begin < 0 {
                        begin_position = begin + len_value + 1;
                    };

                    let mut end_position: i32 = end;

                    if end < 0 {
                        end_position = end + len_value + 1;
                    }

                    let begin_position: usize = begin_position as usize;
                    let end_position: usize = end_position as usize;
                    let return_value = value.get(begin_position..end_position);
                    if return_value.is_none() {
                        return Ok(Response::Normal(Re::List(vec![])));
                    }

                    Ok(Response::Normal(Re::List(return_value.unwrap().to_vec())))
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => Ok(Response::Normal(Re::List(vec![]))),
        }
    }

    fn lrem_method(
        &mut self,
        key: String,
        count: i32,
        element: String,
    ) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command LREM Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                Re::List(value) => match count.cmp(&0) {
                    Ordering::Greater => {
                        let (final_vector, deleted) =
                            Self::remove_repeats(count as usize, element, value.clone());
                        self.db.insert(key.clone(), Re::List(final_vector));
                        Ok(Response::Normal(Re::String(deleted.to_string())))
                    }
                    Ordering::Less => {
                        value.reverse();
                        let (mut final_vector, deleted) =
                            Self::remove_repeats(count as usize, element, value.clone());
                        final_vector.reverse();
                        self.db.insert(key.clone(), Re::List(final_vector));
                        Ok(Response::Normal(Re::String(deleted.to_string())))
                    }
                    Ordering::Equal => {
                        let (final_vector, deleted) =
                            Self::remove_all_repeats(element, value.clone());
                        self.db.insert(key.clone(), Re::List(final_vector));
                        Ok(Response::Normal(Re::String(deleted.to_string())))
                    }
                },
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => Ok(Response::Normal(Re::String("0".to_string()))),
        }
    }

    fn remove_repeats(
        count: usize,
        element: String,
        mut vector: Vec<String>,
    ) -> (Vec<String>, usize) {
        let mut n = 0;
        for i in 0..vector.len() {
            if n <= count && vector.get(i).is_some() && *vector.get(i).unwrap() == element {
                vector.remove(i);
                n += 1;
            }
        }
        (vector, n)
    }

    fn remove_all_repeats(element: String, mut vector: Vec<String>) -> (Vec<String>, usize) {
        let mut n = 0;
        for i in 0..vector.len() {
            if vector.get(i).is_some() && *vector.get(i).unwrap() == element {
                vector.remove(i);
                n += 1;
            }
        }
        (vector, n)
    }

    fn lset_method(
        &mut self,
        key: String,
        index: i32,
        element: String,
    ) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command LSET Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                Re::List(value) => {
                    let len_value = value.len() as i32;
                    let mut position: i32 = index;

                    if index < 0 {
                        position = index + len_value;
                    };

                    if position < 0 || position > len_value {
                        let _ = self.log_sender.send(Log::new(
                            LogLevel::Error,
                            line!(),
                            column!(),
                            file!().to_string(),
                            "ERR index out of range".to_string(),
                        ));
                        return Err("ERR index out of range".to_string());
                    }

                    let saved_value = value;

                    saved_value[position as usize] = element;

                    Ok(Response::Normal(Re::String("Ok".to_string())))
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    "ERR no such key".to_string(),
                ));
                Err("ERR no such key".to_string())
            }
        }
    }

    fn rpop_method(&mut self, key: String, count: usize) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command RPOP Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                Re::List(value) => {
                    let return_value: Vec<String>;
                    let mut vector_to_save: Vec<String>;
                    value.reverse();

                    if count == 0 {
                        return_value = Vec::from(value.get(..=count).unwrap());
                        vector_to_save = Vec::from(value.get(count + 1..).unwrap());
                    } else {
                        let qty = match count {
                            x if x >= value.len() => value.len(),
                            _ => count,
                        };
                        return_value = Vec::from(value.get(..qty).unwrap());
                        vector_to_save = Vec::from(value.get(qty..).unwrap());
                    }

                    vector_to_save.reverse();
                    self.db.insert(key, Re::List(vector_to_save));

                    if return_value.len() == 1 && return_value.first().is_some() {
                        let value = return_value.first();
                        return Ok(Response::Normal(Re::String(value.unwrap().to_string())));
                    }

                    match return_value.len() {
                        x if x > 0 => Ok(Response::Normal(Re::List(return_value.to_vec()))),
                        _ => Ok(Response::Normal(Re::Nil)),
                    }
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => Ok(Response::Normal(Re::Nil)),
        }
    }

    fn rpush_method(&mut self, key: String, values: Vec<String>) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command RPUSH Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                Re::List(value) => {
                    let mut saved_vector = value.clone();
                    saved_vector.extend(values);
                    self.db.insert(key, Re::List(saved_vector.clone()));

                    Ok(Response::Normal(Re::String(saved_vector.len().to_string())))
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => {
                self.db.insert(key, Re::List(values.clone()));

                Ok(Response::Normal(Re::String(values.len().to_string())))
            }
        }
    }

    fn rpushx_method(&mut self, key: String, values: Vec<String>) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command RPUSHX Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                RedisElement::List(value) => {
                    let mut saved_vector = value.clone();
                    saved_vector.extend(values);
                    self.db
                        .insert(key, RedisElement::List(saved_vector.clone()));

                    Ok(Response::Normal(Re::String(saved_vector.len().to_string())))
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        WRONGTYPE_MSG.to_string(),
                    ));
                    Err(WRONGTYPE_MSG.to_string())
                }
            },
            None => Ok(Response::Normal(Re::String("0".to_string()))),
        }
    }

    fn sadd_method(&mut self, key: String, values: HashSet<String>) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command SADD Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                RedisElement::Set(value) => {
                    let mut set = value.clone();
                    let start_set_len = set.len();
                    set.extend(values);
                    let final_set_len = set.len();
                    self.db.insert(key, RedisElement::Set(set));

                    Ok(Response::Normal(Re::String(
                        (final_set_len - start_set_len).to_string(),
                    )))
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        "WRONGTYPE A hashset data type expected".to_string(),
                    ));
                    Err("WRONGTYPE A hashset data type expected".to_string())
                }
            },
            None => {
                self.db.insert(key, RedisElement::Set(values.clone()));
                Ok(Response::Normal(Re::String(values.len().to_string())))
            }
        }
    }

    fn scard_method(&mut self, key: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command SCARD Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(value) => match value {
                RedisElement::Set(value) => {
                    let set = value.clone();
                    Ok(Response::Normal(Re::String(set.len().to_string())))
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        "WRONGTYPE A hashset data type expected".to_string(),
                    ));

                    Err("WRONGTYPE A hashset data type expected".to_string())
                }
            },
            None => Ok(Response::Normal(Re::String("0".to_string()))),
        }
    }

    fn sismember_method(&mut self, key: String, value: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command SISMEMBER Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(redis_element) => match redis_element {
                RedisElement::Set(redis_element) => {
                    let set = redis_element.clone();
                    if set.contains(&value) {
                        Ok(Response::Normal(Re::String("1".to_string())))
                    } else {
                        Ok(Response::Normal(Re::String("0".to_string())))
                    }
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        "WRONGTYPE A hashset data type expected".to_string() + &*key,
                    ));

                    Err("WRONGTYPE A hashset data type expected".to_string())
                }
            },
            None => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    "The key doesn't exist".to_string(),
                ));
                Err("The key doesn't exist".to_string())
            }
        }
    }

    fn smembers_method(&mut self, key: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command SMEMBERS Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(redis_element) => match redis_element {
                RedisElement::Set(redis_element) => {
                    Ok(Response::Normal(Re::Set(redis_element.clone())))
                }

                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        "WRONGTYPE A hashset data type expected".to_string(),
                    ));
                    Err("WRONGTYPE A hashset data type expected".to_string())
                }
            },
            None => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    "The key doesn't exist".to_string(),
                ));
                Err("The key doesn't exist".to_string())
            }
        }
    }

    fn srem_method(&mut self, key: String, values: HashSet<String>) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command SREM Received - key: ".to_string() + &*key,
        ));

        match self.db.get_mut(&key) {
            Some(redis_element) => match redis_element {
                RedisElement::Set(redis_element) => {
                    let mut set = redis_element.clone();
                    let mut count = 0;
                    for value in values {
                        if set.remove(&value) {
                            count += 1;
                        }
                    }
                    self.db.insert(key.clone(), RedisElement::Set(set));
                    Ok(Response::Normal(Re::String(count.to_string())))
                }
                _ => {
                    let _ = self.log_sender.send(Log::new(
                        LogLevel::Error,
                        line!(),
                        column!(),
                        file!().to_string(),
                        "WRONGTYPE A hashset data type expected".to_string() + &*key,
                    ));
                    Err("WRONGTYPE A hashset data type expected".to_string())
                }
            },
            None => Ok(Response::Normal(Re::String("0".to_string()))),
        }
    }

    fn keys_method(&mut self, pattern: String) -> Vec<String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command KEYS Received".to_string(),
        ));

        let mut vector = vec![];
        for key in self.db.keys() {
            let re = Regex::new(&*pattern).unwrap();
            if re.is_match(key) {
                vector.push(key.to_string());
            }
        }
        vector
    }

    fn store_method(&self, path: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command STORE Received - path: ".to_string() + &*path,
        ));

        let mut file = match fs::File::create(path) {
            Ok(file) => file,
            Err(e) => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    e.to_string(),
                ));
                return Err(e.to_string());
            }
        };

        match file.write_all(self.db.serialize().as_bytes()) {
            Ok(_) => Ok(Response::Normal(RedisElement::String("Ok".to_string()))),
            Err(e) => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    e.to_string(),
                ));
                Err(e.to_string())
            }
        }
    }

    fn load_method(&mut self, path: String) -> Result<Response, String> {
        let _ = self.log_sender.send(Log::new(
            LogLevel::Debug,
            line!(),
            column!(),
            file!().to_string(),
            "Command LOAD Received - path: ".to_string() + &*path,
        ));

        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    e.to_string(),
                ));
                return Err(e.to_string());
            }
        };

        match TtlHashMap::deserialize(text) {
            Ok(map) => {
                self.db = map;
                Ok(Response::Normal(RedisElement::String("Ok".to_string())))
            }
            Err(e) => {
                let _ = self.log_sender.send(Log::new(
                    LogLevel::Error,
                    line!(),
                    column!(),
                    file!().to_string(),
                    e.clone(),
                ));
                Err(e)
            }
        }
    }
}

#[allow(unused_imports)]
mod test {
    use crate::entities::command::Command;
    use crate::service::redis::{Re, Redis, Response};
    use std::collections::HashSet;
    use std::fs;
    use std::io::Write;
    use std::thread::{self, sleep};
    use std::time::{Duration, SystemTime};

    #[allow(dead_code)]
    fn eq_response(content: Re, response: Response) -> bool {
        if let Response::Normal(redis_element) = response {
            return content == redis_element;
        };
        false
    }

    #[test]
    fn test_strlen_element_fail_if_is_not_string() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let strlen = redis.execute(Command::Strlen { key });

        assert!(strlen.is_err());
    }

    #[ignore]
    #[test]
    fn test_strlen_element_not_found() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let strlen = redis.execute(Command::Strlen { key });

        assert!(strlen.is_ok());
        assert!(eq_response(Re::String("0".to_string()), strlen.unwrap()));
    }

    #[test]
    fn test_strlen_element_saved_before() {
        let mut redis: Redis = Redis::new_for_test();

        let value = "value".to_string();
        let key = "hola".to_string();

        let _set = redis.execute(Command::Set { key, value });

        let key = "hola".to_string();
        let strlen = redis.execute(Command::Strlen { key });

        assert!(strlen.is_ok());
        assert!(eq_response(Re::String("5".to_string()), strlen.unwrap()));
    }

    #[allow(unused_imports)]
    #[test]
    fn test_set_element_and_get_the_same() {
        let mut redis: Redis = Redis::new_for_test();

        let value = "value".to_string();
        let key = "hola".to_string();

        let _set = redis.execute(Command::Set { key, value });

        let key = "hola".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(get.is_ok());
        assert!(eq_response(Re::String("value".to_string()), get.unwrap()));
    }

    #[test]
    fn test_set_element_twice_and_get_the_last_set() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "hola".to_string();
        let value = "chau".to_string();

        let _set = redis.execute(Command::Set { key, value });

        let key = "hola".to_string();
        let value = "test".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "hola".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(get.is_ok());
        assert!(eq_response(Re::String("test".to_string()), get.unwrap()));
    }

    #[test]
    fn test_get_on_empty_key_returns_nil() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "hola".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(get.is_ok());
        assert!(eq_response(Re::Nil, get.unwrap()));
    }

    #[test]
    fn test_get_element_fail_if_is_not_string() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(get.is_err());
    }

    #[test]
    fn test_getset_fails_if_is_not_string() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let value = "value".to_string();
        let getset = redis.execute(Command::Getset { key, value });

        assert!(getset.is_err());
    }

    #[test]
    fn test_getset_on_empty_key_returns_nil() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let getset = redis.execute(Command::Getset { key, value });

        assert!(getset.is_ok());
        assert!(eq_response(Re::Nil, getset.unwrap()));
    }

    #[test]
    fn test_getset_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "1".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let value = "value".to_string();
        let getset = redis.execute(Command::Getset { key, value });
        assert!(getset.is_ok());
        assert!(eq_response(Re::String("1".to_string()), getset.unwrap()));

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(get.is_ok());
        assert!(eq_response(Re::String("value".to_string()), get.unwrap()));
    }

    #[test]
    fn test_ping_returns_pong() {
        let mut redis: Redis = Redis::new_for_test();

        let ping = redis.execute(Command::Ping);

        assert!(ping.is_ok());
        assert!(eq_response(Re::String("PONG".to_string()), ping.unwrap()));
    }

    #[test]
    fn test_incrby_with_2_as_value() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "1".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let increment: u32 = 1;
        let _incrby = redis.execute(Command::Incrby { key, increment });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });

        let key = "key".to_string();
        let increment: u32 = 2;
        let _incrby = redis.execute(Command::Incrby { key, increment });

        let key = "key".to_string();
        let second_get = redis.execute(Command::Get { key });

        assert!(get.is_ok());
        assert!(eq_response(Re::String("2".to_string()), get.unwrap()));

        assert!(second_get.is_ok());
        assert!(eq_response(
            Re::String("4".to_string()),
            second_get.unwrap(),
        ));
    }

    #[test]
    fn test_incrby_value_err_initial_value_string() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "hola".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let increment: u32 = 1;
        let incrby = redis.execute(Command::Incrby { key, increment });

        assert!(incrby.is_err());
    }

    #[test]
    fn test_incrby_not_saved_value() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let increment: u32 = 1;
        let _incrby = redis.execute(Command::Incrby { key, increment });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(get.is_ok());
        assert!(eq_response(Re::String("1".to_string()), get.unwrap()));
    }

    #[test]
    fn test_decrby_on_new_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let decrement: u32 = 3;
        let _decrby = redis.execute(Command::Decrby { key, decrement });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(get.is_ok());
        assert!(eq_response(Re::String("-3".to_string()), get.unwrap()));
    }

    #[test]
    fn test_decrby_on_existing_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "5".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let decrement: u32 = 3;
        let _decrby = redis.execute(Command::Decrby { key, decrement });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(get.is_ok());
        assert!(eq_response(Re::String("2".to_string()), get.unwrap()));
    }

    #[test]
    fn test_mset_sets_2_values() {
        let mut redis: Redis = Redis::new_for_test();

        let key_values = vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ];
        let _mset = redis.execute(Command::Mset { key_values });

        let key = "key1".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(get.is_ok());
        assert!(eq_response(Re::String("value1".to_string()), get.unwrap()));

        let key = "key2".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(get.is_ok());
        assert!(eq_response(Re::String("value2".to_string()), get.unwrap()));
    }

    #[test]
    fn test_mget_gets_2_values() {
        let mut redis: Redis = Redis::new_for_test();

        let key_values = vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ];
        let _mset = redis.execute(Command::Mset { key_values });

        let keys = vec!["key1".to_string(), "key2".to_string()];
        let mget = redis.execute(Command::Mget { keys });

        assert!(mget.is_ok());
        assert!(eq_response(
            Re::List(vec!["value1".to_string(), "value2".to_string()]),
            mget.unwrap(),
        ));
    }

    #[test]
    fn test_mget_nil_for_missing_value() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let keys = vec!["key".to_string(), "key_empty".to_string()];
        let mget = redis.execute(Command::Mget { keys });

        assert!(mget.is_ok());
        assert!(eq_response(
            Re::List(vec!["value".to_string(), "(nil)".to_string()]),
            mget.unwrap(),
        ));
    }

    #[test]
    fn test_mget_nil_for_non_string_value() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key_list".to_string();
        let value = vec!["value1".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let keys = vec!["key".to_string(), "key_list".to_string()];
        let mget = redis.execute(Command::Mget { keys });

        assert!(mget.is_ok());
        assert!(eq_response(
            Re::List(vec!["value".to_string(), "(nil)".to_string()]),
            mget.unwrap(),
        ));
    }

    #[test]
    fn test_set_element_and_getdel() {
        let mut redis: Redis = Redis::new_for_test();

        let value = "value".to_string();
        let key = "key".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });

        let key = "key".to_string();
        let getdel = redis.execute(Command::Getdel { key });

        assert!(get.is_ok());
        assert!(eq_response(Re::String("value".to_string()), get.unwrap()));

        assert!(getdel.is_ok());
        assert!(eq_response(
            Re::String("value".to_string()),
            getdel.unwrap(),
        ));

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(eq_response(Re::Nil, get.unwrap()));
    }

    #[test]
    fn test_getdel_without_previews_saving_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let getdel = redis.execute(Command::Getdel { key });

        assert!(eq_response(Re::Nil, getdel.unwrap()));
    }

    #[test]
    fn test_dbsize() {
        let mut redis: Redis = Redis::new_for_test();

        let dbsize = redis.execute(Command::Dbsize);
        assert!(eq_response(Re::String("0".to_string()), dbsize.unwrap()));

        let value = "value".to_string();
        let key = "key".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let dbsize = redis.execute(Command::Dbsize);
        assert!(eq_response(Re::String("1".to_string()), dbsize.unwrap()));

        let key = "key".to_string();
        let _getdel = redis.execute(Command::Getdel { key });

        let dbsize = redis.execute(Command::Dbsize);
        assert!(eq_response(Re::String("0".to_string()), dbsize.unwrap()));
    }

    #[test]
    fn test_set_element_and_del() {
        let mut redis: Redis = Redis::new_for_test();

        let value = "value".to_string();
        let key = "key".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let keys = vec!["key".to_string()];
        let del = redis.execute(Command::Del { keys });
        assert!(eq_response(Re::String("1".to_string()), del.unwrap()));

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(eq_response(Re::Nil, get.unwrap()));
    }

    #[test]
    fn test_set_two_elements_and_del_both() {
        let mut redis: Redis = Redis::new_for_test();

        let value = "value".to_string();
        let key = "key1".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let value = "value".to_string();
        let key = "key2".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let keys = vec!["key1".to_string(), "key2".to_string()];
        let del = redis.execute(Command::Del { keys });

        assert!(eq_response(Re::String("2".to_string()), del.unwrap()));
    }

    #[test]
    fn test_append_adds_word() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let value = " appended".to_string();
        let _append = redis.execute(Command::Append { key, value });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(eq_response(
            Re::String("value appended".to_string()),
            get.unwrap(),
        ));
    }

    #[test]
    fn test_append_on_non_existent_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = " appended".to_string();
        let _append = redis.execute(Command::Append { key, value });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(eq_response(
            Re::String(" appended".to_string()),
            get.unwrap(),
        ));
    }

    #[test]
    fn test_set_two_elements_and_check_exists_equal_2() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key1".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key2".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let keys = vec!["key1".to_string(), "key2".to_string()];
        let exists = redis.execute(Command::Exists { keys });
        assert!(eq_response(Re::String("2".to_string()), exists.unwrap()));

        let keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
        let exists = redis.execute(Command::Exists { keys });
        assert!(eq_response(Re::String("2".to_string()), exists.unwrap()));
    }

    #[test]
    fn test_copy_on_existing_key_returns_0() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key1".to_string();
        let value = "value1".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key2".to_string();
        let value = "value2".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key_origin: String = "key1".to_string();
        let key_destination: String = "key2".to_string();
        let copy = redis.execute(Command::Copy {
            key_destination,
            key_origin,
        });

        assert!(eq_response(Re::String("0".to_string()), copy.unwrap()));
    }

    #[test]
    fn test_copy_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key1".to_string();
        let value = "value1".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key_origin: String = "key1".to_string();
        let key_destination: String = "key2".to_string();
        let _copy = redis.execute(Command::Copy {
            key_destination,
            key_origin,
        });

        let key = "key2".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(eq_response(Re::String("value1".to_string()), get.unwrap()));
    }

    #[ignore]
    #[test]
    fn test_expire_deletes_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let ttl = Duration::from_secs(1);
        let expire = redis.execute(Command::Expire { key, ttl });

        thread::sleep(Duration::from_secs(1));

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(eq_response(Re::Nil, get.unwrap()));
        assert!(eq_response(Re::String("1".to_string()), expire.unwrap()));
    }

    #[test]
    fn test_expire_returns_0_on_unexisting_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let ttl = Duration::from_secs(1);
        let expire = redis.execute(Command::Expire { key, ttl });

        assert!(eq_response(Re::String("0".to_string()), expire.unwrap()));
    }

    #[test]
    fn test_expireat_with_past_time_deletes_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let ttl = SystemTime::UNIX_EPOCH + Duration::from_secs(1623793215);
        let expire = redis.execute(Command::Expireat { key, ttl });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(eq_response(Re::Nil, get.unwrap()));
        assert!(eq_response(Re::String("1".to_string()), expire.unwrap()));
    }

    #[test]
    fn test_expireat_returns_0_on_unexisting_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let ttl = SystemTime::UNIX_EPOCH + Duration::from_secs(1623793215);
        let expire = redis.execute(Command::Expireat { key, ttl });

        assert!(eq_response(Re::String("0".to_string()), expire.unwrap()));
    }

    #[ignore]
    #[test]
    fn test_persist_deletes_expire_time() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let ttl = Duration::from_secs(1);
        let _expire = redis.execute(Command::Expire { key, ttl });

        let key = "key".to_string();
        let persist = redis.execute(Command::Persist { key });

        thread::sleep(Duration::from_secs(1));

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(eq_response(Re::String("1".to_string()), persist.unwrap()));
        assert!(eq_response(Re::String("value".to_string()), get.unwrap()));
    }

    #[test]
    fn test_persist_returns_0_on_persistent_value() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let persist = redis.execute(Command::Persist { key });

        let key: String = "key".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(eq_response(Re::String("0".to_string()), persist.unwrap()));
        assert!(eq_response(Re::String("value".to_string()), get.unwrap()));
    }

    #[test]
    fn test_persist_returns_0_on_unexisting_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let persist = redis.execute(Command::Persist { key });
        assert!(eq_response(Re::String("0".to_string()), persist.unwrap()));
    }

    #[test]
    fn test_set_and_rename() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key1".to_string();
        let value = "value1".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key_origin: String = "key1".to_string();
        let key_destination: String = "key2".to_string();
        let rename = redis.execute(Command::Rename {
            key_origin,
            key_destination,
        });
        assert!(rename.is_ok());

        let key = "key1".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(eq_response(Re::Nil, get.unwrap()));

        let key = "key2".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(get.is_ok());
        assert!(eq_response(Re::String("value1".to_string()), get.unwrap()));
    }

    #[test]
    fn test_ttl_returns_neg2_on_unexisting_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let ttl = redis.execute(Command::Ttl { key });

        assert!(eq_response(Re::String("-2".to_string()), ttl.unwrap()));
    }

    #[test]
    fn test_ttl_returns_neg1_on_persistent_value() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let ttl = redis.execute(Command::Ttl { key });

        assert!(eq_response(Re::String("-1".to_string()), ttl.unwrap()));
    }

    #[test]
    fn test_ttl_returns_secs_remaining() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let ttl = Duration::from_secs(5);
        let _expire = redis.execute(Command::Expire { key, ttl });

        let key = "key".to_string();
        let ttl = redis.execute(Command::Ttl { key });

        let _key: String = "key".to_string();

        assert!(eq_response(Re::String("4".to_string()), ttl.unwrap()));
    }

    #[test]
    fn test_type_on_string() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let type_method = redis.execute(Command::Type { key });
        assert!(eq_response(
            Re::String("string".to_string()),
            type_method.unwrap(),
        ));
    }

    #[test]
    fn test_type_on_empty_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let type_method = redis.execute(Command::Type { key });

        assert!(eq_response(
            Re::String("none".to_string()),
            type_method.unwrap(),
        ));
    }

    #[test]
    fn test_type_on_list() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let type_method = redis.execute(Command::Type { key });
        assert!(eq_response(
            Re::String("list".to_string()),
            type_method.unwrap(),
        ));
    }

    #[test]
    fn test_type_on_set() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value".to_string());
        let _sadd = redis.execute(Command::Sadd { key, values });

        let key = "key".to_string();
        let type_method = redis.execute(Command::Type { key });
        assert!(eq_response(
            Re::String("set".to_string()),
            type_method.unwrap(),
        ));
    }

    #[test]
    fn test_lindex_with_key_used_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let index = 1;
        let lindex = redis.execute(Command::Lindex { key, index });

        assert!(lindex.is_err());
    }

    #[test]
    fn test_lindex_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let index = 0;
        let lindex = redis.execute(Command::Lindex { key, index });

        assert!(lindex.is_ok());
        assert!(eq_response(
            Re::String("value2".to_string()),
            lindex.unwrap(),
        ));
    }

    #[test]
    fn test_lindex_negative_index_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let index = -1;
        let lindex = redis.execute(Command::Lindex { key, index });

        assert!(lindex.is_ok());
        assert!(eq_response(
            Re::String("value".to_string()),
            lindex.unwrap(),
        ));
    }

    #[test]
    fn test_lindex_negative_index_result_nil_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let index = -3;
        let lindex = redis.execute(Command::Lindex { key, index });

        assert!(lindex.is_ok());
        assert!(eq_response(Re::Nil, lindex.unwrap()));
    }

    #[test]
    fn test_llen_key_saved_as_string_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let llen = redis.execute(Command::Llen { key });

        assert!(llen.is_err());
    }

    #[test]
    fn test_llen_key_not_found_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let llen = redis.execute(Command::Llen { key });

        assert!(llen.is_ok());
        assert!(eq_response(Re::String("0".to_string()), llen.unwrap()));
    }

    #[test]
    fn test_llen_key_used_twice_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let llen = redis.execute(Command::Llen { key });

        assert!(eq_response(Re::String("4".to_string()), llen.unwrap()));
    }

    #[test]
    fn test_lpop_without_count_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let lpop = redis.execute(Command::Lpop { key, count: 0 });
        assert!(lpop.is_ok());
        assert!(eq_response(Re::String("value2".to_string()), lpop.unwrap()));

        let key = "key".to_string();
        let llen = redis.execute(Command::Llen { key });
        assert!(llen.is_ok());
        assert!(eq_response(Re::String("1".to_string()), llen.unwrap()));
    }

    #[test]
    fn test_lpop_with_count_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "value".to_string(),
            "value2".to_string(),
            "value3".to_string(),
            "value4".to_string(),
        ];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let lpop = redis.execute(Command::Lpop { key, count: 2 });
        assert!(lpop.is_ok());
        assert!(eq_response(
            Re::List(vec!["value4".to_string(), "value3".to_string()]),
            lpop.unwrap(),
        ));

        let key = "key".to_string();
        let llen = redis.execute(Command::Llen { key });
        assert!(llen.is_ok());
        assert!(eq_response(Re::String("2".to_string()), llen.unwrap()));
    }

    #[test]
    fn test_lpop_with_count_major_than_len_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "value".to_string(),
            "value2".to_string(),
            "value3".to_string(),
            "value4".to_string(),
        ];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let lpop = redis.execute(Command::Lpop { key, count: 5 });
        assert!(lpop.is_ok());
        assert!(eq_response(
            Re::List(vec![
                "value4".to_string(),
                "value3".to_string(),
                "value2".to_string(),
                "value".to_string()
            ]),
            lpop.unwrap(),
        ));

        let key = "key".to_string();
        let llen = redis.execute(Command::Llen { key });
        assert!(llen.is_ok());
        assert!(eq_response(Re::String("0".to_string()), llen.unwrap()));

        let key = "key".to_string();
        let lpop = redis.execute(Command::Lpop { key, count: 5 });
        assert!(lpop.is_ok());
        assert!(eq_response(Re::Nil, lpop.unwrap()));
    }

    #[test]
    fn test_lpop_with_saved_string_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let lpop = redis.execute(Command::Lpop { key, count: 5 });
        assert!(lpop.is_err());
    }

    #[test]
    fn test_lrange_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "value1".to_string(),
            "value2".to_string(),
            "value3".to_string(),
            "value4".to_string(),
        ];

        let _lpush = redis.execute(Command::Lpush { key, value });
        let key = "key".to_string();
        let lrange = redis.execute(Command::Lrange {
            key,
            begin: 0,
            end: -1,
        });

        assert!(lrange.is_ok());
        assert!(eq_response(
            Re::List(vec![
                "value4".to_string(),
                "value3".to_string(),
                "value2".to_string(),
                "value1".to_string()
            ]),
            lrange.unwrap(),
        ));
    }

    #[test]
    fn test_lrange_ranges_incorrect_return_empty_vec_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "value1".to_string(),
            "value2".to_string(),
            "value3".to_string(),
            "value4".to_string(),
        ];

        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let lrange = redis.execute(Command::Lrange {
            key,
            begin: -1,
            end: 0,
        });

        assert!(lrange.is_ok());
        assert!(eq_response(Re::List(vec![]), lrange.unwrap()));
    }

    #[test]
    fn test_lrange_using_ranges_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "value1".to_string(),
            "value2".to_string(),
            "value3".to_string(),
            "value4".to_string(),
        ];

        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let lrange = redis.execute(Command::Lrange {
            key,
            begin: 2,
            end: 4,
        });

        assert!(lrange.is_ok());
        assert!(eq_response(
            Re::List(vec!["value2".to_string(), "value1".to_string(), ]),
            lrange.unwrap(),
        ));
    }

    #[test]
    fn test_lrange_for_string_value_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value1".to_string();

        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let lrange = redis.execute(Command::Lrange {
            key,
            begin: 2,
            end: 4,
        });

        assert!(lrange.is_err());
    }

    #[test]
    fn test_lset_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let index = -1;
        let element = "Nuevos".to_string();
        let lset = redis.execute(Command::Lset {
            key,
            index,
            element,
        });

        assert!(lset.is_ok());
        assert!(eq_response(Re::String("Ok".to_string()), lset.unwrap()));

        let key = "key".to_string();
        let lrange = redis.execute(Command::Lrange {
            key,
            begin: 0,
            end: -1,
        });

        assert!(lrange.is_ok());
        assert!(eq_response(
            Re::List(vec!["value2".to_string(), "Nuevos".to_string(), ]),
            lrange.unwrap(),
        ));
    }

    #[test]
    fn test_lset_out_of_range_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let index = -50;
        let element = "Nuevos".to_string();
        let lset = redis.execute(Command::Lset {
            key,
            index,
            element,
        });

        assert!(lset.is_err());
    }

    #[test]
    fn test_lset_out_of_range_upper_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let index = 70;
        let element = "Nuevos".to_string();
        let lset = redis.execute(Command::Lset {
            key,
            index,
            element,
        });

        assert!(lset.is_err());
    }

    #[test]
    fn test_lset_key_not_found_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let index = 70;
        let element = "Nuevos".to_string();
        let lset = redis.execute(Command::Lset {
            key,
            index,
            element,
        });

        assert!(lset.is_err());
    }

    #[test]
    fn test_lset_value_saved_was_string_err() {
        let mut redis: Redis = Redis::new_for_test();

        let value = "value".to_string();
        let key = "key".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let index = 70;
        let element = "Nuevos".to_string();
        let lset = redis.execute(Command::Lset {
            key,
            index,
            element,
        });

        assert!(lset.is_err());
    }

    #[test]
    fn test_rpop_without_count_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let rpop = redis.execute(Command::Rpop { key, count: 0 });
        assert!(rpop.is_ok());
        assert!(eq_response(Re::String("value".to_string()), rpop.unwrap()));

        let key = "key".to_string();
        let llen = redis.execute(Command::Llen { key });
        assert!(llen.is_ok());
        assert!(eq_response(Re::String("1".to_string()), llen.unwrap()));
    }

    #[test]
    fn test_rpop_with_count_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "value".to_string(),
            "value2".to_string(),
            "value3".to_string(),
            "value4".to_string(),
        ];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let rpop = redis.execute(Command::Rpop { key, count: 2 });
        assert!(rpop.is_ok());
        assert!(eq_response(
            Re::List(vec!["value".to_string(), "value2".to_string(), ]),
            rpop.unwrap(),
        ));

        let key = "key".to_string();
        let llen = redis.execute(Command::Llen { key });
        assert!(llen.is_ok());
        assert!(eq_response(Re::String("2".to_string()), llen.unwrap()));
    }

    #[test]
    fn test_rpop_with_count_major_than_len_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "value".to_string(),
            "value2".to_string(),
            "value3".to_string(),
            "value4".to_string(),
        ];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let rpop = redis.execute(Command::Rpop { key, count: 5 });
        assert!(rpop.is_ok());
        assert!(eq_response(
            Re::List(vec![
                "value".to_string(),
                "value2".to_string(),
                "value3".to_string(),
                "value4".to_string()
            ]),
            rpop.unwrap(),
        ));

        let key = "key".to_string();
        let llen = redis.execute(Command::Llen { key });
        assert!(llen.is_ok());
        assert!(eq_response(Re::String("0".to_string()), llen.unwrap()));

        let key = "key".to_string();
        let rpop = redis.execute(Command::Rpop { key, count: 5 });
        assert!(rpop.is_ok());
        assert!(eq_response(Re::Nil, rpop.unwrap()));
    }

    #[test]
    fn test_rpop_with_saved_string_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let rpop = redis.execute(Command::Rpop { key, count: 5 });
        assert!(rpop.is_err());
    }

    #[test]
    fn test_lpush_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpush = redis.execute(Command::Lpush { key, value });

        assert!(lpush.is_ok());
        assert!(eq_response(Re::String("2".to_string()), lpush.unwrap()));
    }

    #[test]
    fn test_lpush_with_key_used_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpush = redis.execute(Command::Lpush { key, value });

        assert!(lpush.is_err());
    }

    #[test]
    fn test_lpush_key_used_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpush = redis.execute(Command::Lpush { key, value });

        assert!(lpush.is_ok());
        assert!(eq_response(Re::String("2".to_string()), lpush.unwrap()));

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpush = redis.execute(Command::Lpush { key, value });

        assert!(lpush.is_ok());
        assert!(eq_response(Re::String("4".to_string()), lpush.unwrap()));
    }

    #[test]
    fn test_lpush_key_used_check_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["1".to_string(), "2".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let value = vec!["3".to_string(), "4".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();
        let index = -1;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert!(eq_response(Re::String("1".to_string()), lindex.unwrap()));
        let key = "key".to_string();
        let index = -2;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert!(eq_response(Re::String("2".to_string()), lindex.unwrap()));
        let key = "key".to_string();
        let index = -3;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert!(eq_response(Re::String("3".to_string()), lindex.unwrap()));
        let key = "key".to_string();
        let index = -4;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert!(eq_response(Re::String("4".to_string()), lindex.unwrap()));
    }

    #[test]
    fn test_rpush_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let rpush = redis.execute(Command::Rpush { key, value });

        assert!(rpush.is_ok());
        assert!(eq_response(Re::String("2".to_string()), rpush.unwrap()));
    }

    #[test]
    fn test_rpush_with_key_used_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let rpush = redis.execute(Command::Rpush { key, value });

        assert!(rpush.is_err());
    }

    #[test]
    fn test_rpush_key_used_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let rpush = redis.execute(Command::Rpush { key, value });

        assert!(rpush.is_ok());
        assert!(eq_response(Re::String("2".to_string()), rpush.unwrap()));

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let rpush = redis.execute(Command::Rpush { key, value });

        assert!(rpush.is_ok());
        assert!(eq_response(Re::String("4".to_string()), rpush.unwrap()));
    }

    #[test]
    fn test_rpush_key_used_check_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["1".to_string(), "2".to_string()];
        let _rpush = redis.execute(Command::Rpush { key, value });

        let key = "key".to_string();
        let value = vec!["3".to_string(), "4".to_string()];
        let _rpush = redis.execute(Command::Rpush { key, value });

        let key = "key".to_string();
        let index = -1;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert!(eq_response(Re::String("4".to_string()), lindex.unwrap()));
        let key = "key".to_string();
        let index = -2;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert!(eq_response(Re::String("3".to_string()), lindex.unwrap()));
        let key = "key".to_string();
        let index = -3;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert!(eq_response(Re::String("2".to_string()), lindex.unwrap()));
        let key = "key".to_string();
        let index = -4;
        let lindex = redis.execute(Command::Lindex { key, index });
        assert!(lindex.is_ok());
        assert!(eq_response(Re::String("1".to_string()), lindex.unwrap()));
    }

    #[test]
    fn test_sadd() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let sadd = redis.execute(Command::Sadd { key, values });

        assert!(eq_response(Re::String("3".to_string()), sadd.unwrap()));
    }

    #[test]
    fn test_sadd_with_existing_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "set".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let sadd = redis.execute(Command::Sadd { key, values });

        assert!(eq_response(Re::String("3".to_string()), sadd.unwrap()));

        let key = "set".to_string();
        let mut values = HashSet::new();
        values.insert("value3".to_string());
        values.insert("value4".to_string());

        let sadd2 = redis.execute(Command::Sadd { key, values });
        assert!(eq_response(Re::String("1".to_string()), sadd2.unwrap()));
    }

    #[test]
    fn test_sadd_error() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "set".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "set".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let sadd = redis.execute(Command::Sadd { key, values });

        assert_eq!(
            "WRONGTYPE A hashset data type expected".to_string(),
            sadd.err().unwrap()
        )
    }

    #[test]
    fn test_scard() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let _sadd = redis.execute(Command::Sadd { key, values });

        let key = "key".to_string();
        let scard = redis.execute(Command::Scard { key });

        assert!(eq_response(Re::String("3".to_string()), scard.unwrap()));
    }

    #[test]
    fn test_scard_error() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "set".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "set".to_string();
        let scard = redis.execute(Command::Scard { key });

        assert_eq!(
            "WRONGTYPE A hashset data type expected".to_string(),
            scard.err().unwrap()
        )
    }

    #[test]
    fn test_sismember() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let _sadd = redis.execute(Command::Sadd { key, values });

        let key = "key".to_string();
        let value = "value1".to_string();
        let sismember = redis.execute(Command::Sismember { key, value });

        assert!(eq_response(Re::String("1".to_string()), sismember.unwrap()));

        let key = "key".to_string();
        let value = "value".to_string();
        let sismember = redis.execute(Command::Sismember { key, value });

        assert!(eq_response(Re::String("0".to_string()), sismember.unwrap()));
    }

    #[test]
    fn test_sismember_error() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let _sadd = redis.execute(Command::Sadd { key, values });

        let key = "key1".to_string();
        let value = "value1".to_string();
        let sismember = redis.execute(Command::Sismember { key, value });

        assert_eq!(
            "The key doesn't exist".to_string(),
            sismember.err().unwrap()
        );

        let key = "set".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "set".to_string();
        let value = "value".to_string();
        let sismember = redis.execute(Command::Sismember { key, value });

        assert_eq!(
            "WRONGTYPE A hashset data type expected".to_string(),
            sismember.err().unwrap()
        );
    }

    #[test]
    fn test_srem() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let _sadd = redis.execute(Command::Sadd { key, values });

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        let srem = redis.execute(Command::Srem { key, values });

        assert!(eq_response(Re::String("1".to_string()), srem.unwrap()));

        let key = "key_inexistente".to_string();
        let mut values = HashSet::new();
        values.insert("value2".to_string());
        let srem = redis.execute(Command::Srem { key, values });

        assert!(eq_response(Re::String("0".to_string()), srem.unwrap()));
    }

    #[test]
    fn test_srem_value_two_times() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let _sadd = redis.execute(Command::Sadd { key, values });

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        let srem = redis.execute(Command::Srem { key, values });

        assert!(eq_response(Re::String("1".to_string()), srem.unwrap()));

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        let srem = redis.execute(Command::Srem { key, values });

        assert!(eq_response(Re::String("0".to_string()), srem.unwrap()));
    }

    #[test]
    fn test_srem_error() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "set".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "set".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        let srem = redis.execute(Command::Srem { key, values });

        assert_eq!(
            "WRONGTYPE A hashset data type expected".to_string(),
            srem.err().unwrap()
        );
    }

    #[test]
    fn test_smembers() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let _sadd = redis.execute(Command::Sadd { key, values });

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        values.insert("value3".to_string());
        let smembers = redis.execute(Command::Smembers { key });

        assert!(eq_response(Re::Set(values), smembers.unwrap()));
    }

    #[test]
    fn test_lpushx_not_pre_save_return_0() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpushx = redis.execute(Command::Lpushx { key, value });

        assert!(lpushx.is_ok());
        assert!(eq_response(Re::String("0".to_string()), lpushx.unwrap()));
    }

    #[test]
    fn test_lpushx_with_key_used_with_string_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpushx = redis.execute(Command::Lpushx { key, value });

        assert!(lpushx.is_err());
    }

    #[test]
    fn test_lpushx_after_lpush_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpush = redis.execute(Command::Lpush { key, value });

        assert!(lpush.is_ok());
        assert!(eq_response(Re::String("2".to_string()), lpush.unwrap()));

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let lpush = redis.execute(Command::Lpushx { key, value });

        assert!(lpush.is_ok());
        assert!(eq_response(Re::String("4".to_string()), lpush.unwrap()));
    }

    #[test]
    fn test_rpushx_not_pre_save_return_0() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let rpushx = redis.execute(Command::Rpushx { key, value });

        assert!(rpushx.is_ok());
        assert!(eq_response(Re::String("0".to_string()), rpushx.unwrap()));
    }

    #[test]
    fn test_rpushx_with_key_used_with_string_err() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let rpushx = redis.execute(Command::Rpushx { key, value });

        assert!(rpushx.is_err());
    }

    #[test]
    fn test_rpushx_after_rpush_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let rpushx = redis.execute(Command::Rpush { key, value });

        assert!(rpushx.is_ok());
        assert!(eq_response(Re::String("2".to_string()), rpushx.unwrap()));

        let key = "key".to_string();
        let value = vec!["value".to_string(), "value2".to_string()];
        let rpushx = redis.execute(Command::Rpushx { key, value });

        assert!(rpushx.is_ok());
        assert!(eq_response(Re::String("4".to_string()), rpushx.unwrap()));
    }

    #[test]
    fn test_rpush_and_check_elements_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
        ];
        let rpushx = redis.execute(Command::Rpush { key, value });

        assert!(rpushx.is_ok());
        assert!(eq_response(Re::String("4".to_string()), rpushx.unwrap()));

        let key = "key".to_string();
        let value = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
        ];
        let rpushx = redis.execute(Command::Lrange {
            key,
            begin: 0,
            end: -1,
        });

        assert!(rpushx.is_ok());
        assert!(eq_response(Re::List(value), rpushx.unwrap()));
    }

    #[test]
    fn test_rpush_rpushx_and_check_elements_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["1".to_string(), "2".to_string()];
        let rpush = redis.execute(Command::Rpush { key, value });

        assert!(rpush.is_ok());
        assert!(eq_response(Re::String("2".to_string()), rpush.unwrap()));

        let key = "key".to_string();
        let value = vec!["3".to_string(), "4".to_string()];
        let rpushx = redis.execute(Command::Rpushx { key, value });

        assert!(rpushx.is_ok());
        assert!(eq_response(Re::String("4".to_string()), rpushx.unwrap()));

        let key = "key".to_string();
        let rpushx = redis.execute(Command::Lrange {
            key,
            begin: 0,
            end: -1,
        });

        assert!(rpushx.is_ok());
        assert!(eq_response(
            Re::List(vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string()
            ]),
            rpushx.unwrap(),
        ));
    }

    #[test]
    fn test_lrem_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "value".to_string(),
            "value1".to_string(),
            "value2".to_string(),
            "value".to_string(),
            "value".to_string(),
        ];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();

        let lrem = redis.execute(Command::Lrem {
            key,
            count: 2,
            element: "value".to_string(),
        });
        assert!(lrem.is_ok());
        assert!(eq_response(Re::String("2".to_string()), lrem.unwrap()));

        let key = "key".to_string();

        let lrange = redis.execute(Command::Lrange {
            key,
            begin: 0,
            end: -1,
        });

        let mut vector = vec![
            "value1".to_string(),
            "value2".to_string(),
            "value".to_string(),
        ];
        vector.reverse();
        assert!(eq_response(Re::List(vector), lrange.unwrap()));
    }

    #[test]
    fn test_lrem_reverse_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "value".to_string(),
            "value".to_string(),
            "value2".to_string(),
            "value3".to_string(),
            "value1".to_string(),
            "value".to_string(),
        ];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();

        let lrem = redis.execute(Command::Lrem {
            key,
            count: -2,
            element: "value".to_string(),
        });
        assert!(lrem.is_ok());
        assert!(eq_response(Re::String("2".to_string()), lrem.unwrap()));

        let key = "key".to_string();

        let lrange = redis.execute(Command::Lrange {
            key,
            begin: 0,
            end: -1,
        });

        let mut vector = vec![
            "value".to_string(),
            "value2".to_string(),
            "value3".to_string(),
            "value1".to_string(),
        ];

        vector.reverse();

        assert!(eq_response(Re::List(vector), lrange.unwrap()));
    }

    #[test]
    fn test_lrem_all_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec![
            "value".to_string(),
            "value2".to_string(),
            "value3".to_string(),
            "value1".to_string(),
            "value".to_string(),
        ];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let key = "key".to_string();

        let lrem = redis.execute(Command::Lrem {
            key,
            count: 0,
            element: "value".to_string(),
        });
        assert!(lrem.is_ok());
        assert!(eq_response(Re::String("2".to_string()), lrem.unwrap()));

        let key = "key".to_string();

        let lrange = redis.execute(Command::Lrange {
            key,
            begin: 0,
            end: -1,
        });

        let mut vector = vec![
            "value2".to_string(),
            "value3".to_string(),
            "value1".to_string(),
        ];

        vector.reverse();

        assert!(eq_response(Re::List(vector), lrange.unwrap()));
    }

    #[test]
    fn test_lrem_invalid_key_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();

        let lrem = redis.execute(Command::Lrem {
            key,
            count: 0,
            element: "value".to_string(),
        });
        assert!(lrem.is_ok());
        assert!(eq_response(Re::String("0".to_string()), lrem.unwrap()));
    }

    #[test]
    fn test_keys_ok() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key1".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let pattern: String = "/*".to_string();

        let keys = redis.execute(Command::Keys { pattern });

        assert!(keys.is_ok());

        let pattern: String = "k**".to_string();

        let keys = redis.execute(Command::Keys { pattern });

        assert!(keys.is_ok());
    }

    #[ignore]
    #[test]
    fn test_touch_deletes_expired_key() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let ttl = Duration::from_secs(1);
        let _expire = redis.execute(Command::Expire { key, ttl });

        thread::sleep(Duration::from_secs(1));

        let keys = vec!["key".to_string()];
        let touch = redis.execute(Command::Touch { keys });

        let pattern = "*".to_string();
        let keys = redis.execute(Command::Keys { pattern });

        assert!(eq_response(Re::String("0".to_string()), touch.unwrap()));
        assert!(eq_response( Re::List(Vec::new()), keys.unwrap()));
    }

    #[test]
    fn test_touch_returns_number_of_keys_touched() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key1".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key2".to_string();
        let value = "value".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let keys = vec!["key1".to_string(), "key2".to_string()];
        let touch = redis.execute(Command::Touch { keys });

        assert!(eq_response(Re::String("2".to_string()), touch.unwrap()));
    }

    #[test]
    fn test_set_element_and_flushdb() {
        let mut redis: Redis = Redis::new_for_test();

        let value = "value".to_string();
        let key = "key".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(eq_response(Re::String("value".to_string()), get.unwrap()));

        println!("{:?}", redis);

        let flushdb = redis.execute(Command::Flushdb);
        assert!(flushdb.is_ok());

        println!("{:?}", redis);

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });
        assert!(eq_response(Re::Nil, get.unwrap()));
    }

    #[test]
    fn test_store_strings() {
        let mut redis: Redis = Redis::new_for_test();

        let value = "value1".to_string();
        let key = "key1".to_string();
        let _set = redis.execute(Command::Set { key, value });
        let value = "value2".to_string();
        let key = "key2".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let path = "test_store_string.rdb".to_string();
        let _store = redis.execute(Command::Store { path });

        let path = "test_store_string.rdb".to_string();
        let content = fs::read_to_string(path).unwrap();
        assert!(
            content == "key1,value1,0\nkey2,value2,0\n"
                || content == "key2,value2,0\nkey1,value1,0\n"
        );

        fs::remove_file("test_store_string.rdb").unwrap();
    }

    #[test]
    fn test_store_list() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let value = vec!["value2".to_string(), "value1".to_string()];
        let _lpush = redis.execute(Command::Lpush { key, value });

        let path = "test_store_list.rdb".to_string();
        let _store = redis.execute(Command::Store { path });

        let path = "test_store_list.rdb".to_string();
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, "key,[value1 - value2],0\n");

        fs::remove_file("test_store_list.rdb").unwrap();
    }

    #[test]
    fn test_store_set() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        let _sadd = redis.execute(Command::Sadd { key, values });

        let path = "test_store_set.rdb".to_string();
        let _store = redis.execute(Command::Store { path });

        let path = "test_store_set.rdb".to_string();
        let content = fs::read_to_string(path).unwrap();
        assert!(content == "key,{value1 - value2},0\n" || content == "key,{value2 - value1},0\n");

        fs::remove_file("test_store_set.rdb").unwrap();
    }

    #[test]
    fn test_store_string_with_ttl() {
        let mut redis: Redis = Redis::new_for_test();

        let value = "value".to_string();
        let key = "key".to_string();
        let _set = redis.execute(Command::Set { key, value });

        let key = "key".to_string();
        let ttl = Duration::from_secs(2);
        let _expire = redis.execute(Command::Expire { key, ttl });

        let path = "test_store_string_with_ttl.rdb".to_string();
        let _store = redis.execute(Command::Store { path });
        let ttl = (SystemTime::now() + Duration::from_secs(2))
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let path = "test_store_string_with_ttl.rdb".to_string();
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, format!("key,value,{}\n", ttl));

        fs::remove_file("test_store_string_with_ttl.rdb").unwrap();
    }

    #[test]
    fn test_store_value_with_dash() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value - 1".to_string());
        values.insert("value2".to_string());
        let _sadd = redis.execute(Command::Sadd { key, values });

        let path = "test_store_value_with_dash.rdb".to_string();
        let _store = redis.execute(Command::Store { path });

        let path = "test_store_value_with_dash.rdb".to_string();
        let content = fs::read_to_string(path).unwrap();
        assert!(content == "key,{value2 - value-1},0\n" || content == "key,{value-1 - value2},0\n");

        fs::remove_file("test_store_value_with_dash.rdb").unwrap();
    }

    #[test]
    fn test_store_values_with_separated_words() {
        let mut redis: Redis = Redis::new_for_test();

        let key = "key".to_string();
        let mut values = HashSet::new();
        values.insert("value 1".to_string());
        values.insert("value2".to_string());
        let _sadd = redis.execute(Command::Sadd { key, values });

        let path = "test_store_values_with_separated_words.rdb".to_string();
        let _store = redis.execute(Command::Store { path });

        let path = "test_store_values_with_separated_words.rdb".to_string();
        let content = fs::read_to_string(path).unwrap();
        assert!(content == "key,{value2 - value 1},0\n" || content == "key,{value 1 - value2},0\n");

        fs::remove_file("test_store_values_with_separated_words.rdb").unwrap();
    }

    #[test]
    fn test_load_string() {
        let mut file = fs::File::create("test_load_string.rdb").unwrap();
        file.write_all("key,value,0\n".as_bytes()).unwrap();

        let mut redis: Redis = Redis::new_for_test();
        let path = "test_load_string.rdb".to_string();
        let _load = redis.execute(Command::Load { path });

        let key = "key".to_string();
        let get = redis.execute(Command::Get { key });

        assert!(eq_response(Re::String("value".to_string()), get.unwrap()));

        fs::remove_file("test_load_string.rdb").unwrap();
    }

    #[test]
    fn test_load_list() {
        let mut file = fs::File::create("test_load_list.rdb").unwrap();
        file.write_all("key,[value1 - value2],0\n".as_bytes())
            .unwrap();

        let mut redis: Redis = Redis::new_for_test();
        let path = "test_load_list.rdb".to_string();
        let _load = redis.execute(Command::Load { path });

        let key = "key".to_string();
        let index = 0;
        let lindex_0 = redis.execute(Command::Lindex { key, index });
        assert!(eq_response(
            Re::String("value1".to_string()),
            lindex_0.unwrap(),
        ));

        let key = "key".to_string();
        let index = 1;
        let lindex_1 = redis.execute(Command::Lindex { key, index });
        assert!(eq_response(
            Re::String("value2".to_string()),
            lindex_1.unwrap(),
        ));

        fs::remove_file("test_load_list.rdb").unwrap();
    }

    #[test]
    fn test_load_set() {
        let mut file = fs::File::create("test_load_set.rdb").unwrap();
        file.write_all("key,{value1 - value2},0\n".as_bytes())
            .unwrap();

        let mut redis: Redis = Redis::new_for_test();
        let path = "test_load_set.rdb".to_string();
        let _load = redis.execute(Command::Load { path });

        let key = "key".to_string();
        let smembers = redis.execute(Command::Smembers { key });

        let mut values = HashSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());

        assert!(eq_response(Re::Set(values), smembers.unwrap()));

        fs::remove_file("test_load_set.rdb").unwrap();
    }

    #[test]
    fn test_load_string_with_ttl() {
        let mut file = fs::File::create("test_load_string_with_ttl.rdb").unwrap();
        let ttl = (SystemTime::now() + Duration::from_secs(2))
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let serialized = format!("key,value,{}\n", ttl);
        file.write_all(serialized.as_bytes()).unwrap();

        let mut redis: Redis = Redis::new_for_test();
        let path = "test_load_string_with_ttl.rdb".to_string();
        let _load = redis.execute(Command::Load { path });

        let key = "key".to_string();
        let get_ttl = redis.execute(Command::Ttl { key });

        assert!(eq_response(Re::String("1".to_string()), get_ttl.unwrap()));

        fs::remove_file("test_load_string_with_ttl.rdb").unwrap();
    }

    #[test]
    fn test_load_values_with_dash() {
        let mut redis: Redis = Redis::new_for_test();

        let mut file = fs::File::create("test_load_values_with_dash.rdb").unwrap();
        file.write_all("key,{value-1 - value2},0\n".as_bytes())
            .unwrap();

        let path = "test_load_values_with_dash.rdb".to_string();
        let _load = redis.execute(Command::Load { path });

        let key = "key".to_string();
        let smembers = redis.execute(Command::Smembers { key });

        let mut values = HashSet::new();
        values.insert("value-1".to_string());
        values.insert("value2".to_string());

        assert!(eq_response(Re::Set(values), smembers.unwrap()));

        fs::remove_file("test_load_values_with_dash.rdb").unwrap();
    }

    #[test]
    fn test_load_values_with_separated_words() {
        let mut redis: Redis = Redis::new_for_test();

        let mut file = fs::File::create("test_load_values_with_separated_words.rdb").unwrap();
        file.write_all("key,{value 1 - value2},0\n".as_bytes())
            .unwrap();

        let path = "test_load_values_with_separated_words.rdb".to_string();
        let _load = redis.execute(Command::Load { path });

        let key = "key".to_string();
        let smembers = redis.execute(Command::Smembers { key });

        let mut values = HashSet::new();
        values.insert("value 1".to_string());
        values.insert("value2".to_string());

        assert!(eq_response(Re::Set(values), smembers.unwrap()));

        fs::remove_file("test_load_values_with_separated_words.rdb").unwrap();
    }
}

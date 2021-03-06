use crate::entities::command::Command;
use crate::entities::info_param::InfoParam;
use crate::entities::pubsub_param::PubSubParam;
use core::time::Duration;
use std::collections::HashSet;
use std::iter::FromIterator;
use std::time::SystemTime;

#[allow(dead_code)]
/// Generador de comandos validos a partir de listado de strings provenientes del Cliente.
pub fn generate(params: Vec<String>, client_id: String) -> Result<Command, String> {
    if params.is_empty() {
        return Err("Params can't be empty".to_string());
    }

    let command = params.first().unwrap();
    let params = Vec::from(params.get(1..).unwrap());
    match command.to_lowercase().as_str() {
        // Server
        "ping" => generate_ping(params),
        "flushdb" => generate_flushdb(params),
        "dbsize" => generate_dbsize(params),
        "monitor" => generate_monitor(params),
        "info" => generate_info(params),

        "store" => generate_store(params),
        "load" => generate_load(params),
        "config" => generate_config(params),

        // Strings
        "get" => generate_get(params),
        "getset" => generate_getset(params),
        "set" => generate_set(params),
        "incrby" => generate_incrby(params),
        "decrby" => generate_decrby(params),
        "getdel" => generate_getdel(params),
        "append" => generate_append(params),
        "mget" => generate_mget(params),
        "mset" => generate_mset(params),
        "strlen" => generate_strlen(params),

        // Keys
        "copy" => generate_copy(params),
        "del" => generate_del(params),
        "exists" => generate_exists(params),
        "expire" => generate_expire(params),
        "expireat" => generate_expireat(params),
        "persist" => generate_persist(params),
        "rename" => generate_rename(params),
        "sort" => generate_sort(params),
        "touch" => generate_touch(params),
        "ttl" => generate_ttl(params),
        "type" => generate_type(params),

        // Lists
        "lindex" => generate_lindex(params),
        "llen" => generate_llen(params),
        "lpop" => generate_lpop(params),
        "lpush" => generate_lpush(params),
        "lpushx" => generate_lpushx(params),
        "lrange" => generate_lrange(params),
        "lrem" => generate_lrem(params),
        "lset" => generate_lset(params),
        "rpop" => generate_rpop(params),
        "rpush" => generate_rpush(params),
        "rpushx" => generate_rpushx(params),

        //Sets
        "sadd" => generate_sadd(params),
        "scard" => generate_scard(params),
        "sismember" => generate_sismember(params),
        "smembers" => generate_smembers(params),
        "srem" => generate_srem(params),
        "keys" => generate_keys(params),

        //PubSub
        "pubsub" => generate_pubsub(params),
        "subscribe" => generate_subscribe(params, client_id),
        "publish" => generate_publish(params),
        "unsubscribe" => Ok(generate_unsubscribe(params, client_id)),
        "command" => Ok(Command::Command),

        _ => Err("Command not valid".to_string()),
    }
}

/// Generador de comando Command::Ping.
fn generate_ping(params: Vec<String>) -> Result<Command, String> {
    if params.len() > 1 {
        return Err("ERR wrong number of arguments for 'ping' command".to_string());
    }

    Ok(Command::Ping)
}

/// Generador de comando Command::Monitor.
fn generate_monitor(params: Vec<String>) -> Result<Command, String> {
    if params.len() > 1 {
        return Err("ERR wrong number of arguments for 'monitor' command".to_string());
    }

    Ok(Command::Monitor)
}

/// Generador de comando Command::Info
fn generate_info(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 1 {
        return Err("ERR wrong number of arguments for 'info' command".to_string());
    }

    match params[0].to_lowercase().as_str() {
        "processid" => Ok(Command::Info {
            param: InfoParam::ProcessId,
        }),
        "port" => Ok(Command::Info {
            param: InfoParam::Port,
        }),
        "servertime" => Ok(Command::Info {
            param: InfoParam::ServerTime,
        }),
        "uptime" => Ok(Command::Info {
            param: InfoParam::Uptime,
        }),
        "configfile" => Ok(Command::Info {
            param: InfoParam::ConfigFile,
        }),
        "connectedclients" => Ok(Command::Info {
            param: InfoParam::ConnectedClients,
        }),
        _ => Err("ERR wrong command param".to_string()),
    }
}

/// Generador de comando Command::Config
fn generate_config(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'config' command".to_string());
    }

    match params[0].as_str() {
        "set" => {
            if params.len() != 3 {
                return Err("ERR wrong number of arguments for 'config set' command".to_string());
            }
            let parameter = params[1].clone();
            let value = params[2].clone();
            Ok(Command::ConfigSet { parameter, value })
        }
        "get" => Ok(Command::ConfigGet),
        _ => Err("ERR wrong arguments for 'config' command".to_string()),
    }
}

/// Generador de comando Command::Flushdb
fn generate_flushdb(params: Vec<String>) -> Result<Command, String> {
    if params.len() > 1 {
        return Err("ERR wrong number of arguments for 'flushdb' command".to_string());
    }

    Ok(Command::Flushdb)
}
/// Generador de comando Command::Copy
fn generate_copy(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR wrong number of arguments for 'copy' command".to_string());
    }

    let key_origin = params[0].clone();
    let key_destination = params[1].clone();
    Ok(Command::Copy {
        key_origin,
        key_destination,
    })
}

/// Generador de comando Command::Get
fn generate_get(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 1 {
        return Err("ERR wrong number of arguments for 'get' command".to_string());
    }

    let key = params[0].clone();
    Ok(Command::Get { key })
}

/// Generador de comando Command::GetSet
fn generate_getset(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR wrong number of arguments for 'getset' command".to_string());
    }

    let key = params[0].clone();
    let value = params[1].clone();
    Ok(Command::Getset { key, value })
}

/// Generador de comando Command::Set
fn generate_set(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR syntax error".to_string());
    }

    let key = params[0].clone();
    let value = params[1].clone();
    Ok(Command::Set { key, value })
}

/// Generador de comando Command::Incrby
fn generate_incrby(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR syntax error".to_string());
    }

    let key = params[0].clone();
    let increment: Result<u32, _> = params[1].to_string().parse();

    if increment.is_err() {
        return Err("ERR value is not an integer or out of range".to_string());
    }

    let increment = increment.unwrap();
    Ok(Command::Incrby { key, increment })
}

/// Generador de comando Command::Decrby
fn generate_decrby(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR syntax error".to_string());
    }

    let key = params[0].clone();
    let decrement: Result<u32, _> = params[1].to_string().parse();

    if decrement.is_err() {
        return Err("ERR value is not an integer or out of range".to_string());
    }

    let decrement = decrement.unwrap();
    Ok(Command::Decrby { key, decrement })
}

/// Generador de comando Command::GetDel
fn generate_getdel(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 1 {
        return Err("ERR wrong number of arguments for 'getdel' command".to_string());
    }

    let key = params[0].clone();
    Ok(Command::Getdel { key })
}

/// Generador de comando Command::Del
fn generate_del(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'del' command".to_string());
    }

    Ok(Command::Del { keys: params })
}

/// Generador de comando Command::Append
fn generate_append(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR wrong number of arguments for 'append' command".to_string());
    }

    let key = params[0].clone();
    let value = params[1].clone();
    Ok(Command::Append { key, value })
}

/// Generador de comando Command::Exists
fn generate_exists(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'exists' command".to_string());
    }

    Ok(Command::Exists { keys: params })
}

/// Generador de comando Command::Expire
fn generate_expire(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR wrong number of arguments for 'expire' command".to_string());
    }

    let key = params[0].clone();
    let seconds: Result<u32, _> = params[1].to_string().parse();

    if seconds.is_err() {
        return Err("ERR value is not an integer or out of range".to_string());
    }

    let ttl = Duration::from_secs(seconds.unwrap().into());

    Ok(Command::Expire { key, ttl })
}

/// Generador de comando Command::ExpireAt
fn generate_expireat(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR wrong number of arguments for 'expireat' command".to_string());
    }

    let key = params[0].clone();
    let seconds: Result<u32, _> = params[1].to_string().parse();

    if seconds.is_err() {
        return Err("ERR value is not an integer or out of range".to_string());
    }

    let ttl = SystemTime::UNIX_EPOCH + Duration::from_secs(seconds.unwrap().into());

    Ok(Command::Expireat { key, ttl })
}

/// Generador de comando Command::Persist
fn generate_persist(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 1 {
        return Err("ERR wrong number of arguments for 'persist' command".to_string());
    }

    let key = params[0].clone();
    Ok(Command::Persist { key })
}

/// Generador de comando Command::Rename
fn generate_rename(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR wrong number of arguments for 'rename' command".to_string());
    }

    let key_origin = params[0].clone();
    let key_destination = params[1].clone();
    Ok(Command::Rename {
        key_origin,
        key_destination,
    })
}

/// Generador de comando Command::Sort
fn generate_sort(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 1 {
        return Err("ERR wrong number of arguments for 'sort' command".to_string());
    }

    let key = params[0].clone();
    Ok(Command::Sort { key })
}

/// Generador de comando Command::Touch
fn generate_touch(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'touch' command".to_string());
    }

    Ok(Command::Touch { keys: params })
}

/// Generador de comando Command::Ttl
fn generate_ttl(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 1 {
        return Err("ERR wrong number of arguments for 'ttl' command".to_string());
    }

    let key = params[0].clone();
    Ok(Command::Ttl { key })
}

/// Generador de comando Command::Type
fn generate_type(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 1 {
        return Err("ERR wrong number of arguments for 'type' command".to_string());
    }

    let key = params[0].clone();
    Ok(Command::Type { key })
}

/// Generador de comando Command::Mget
fn generate_mget(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'mget' command".to_string());
    }

    Ok(Command::Mget { keys: params })
}

/// Generador de comando Command::Mset
fn generate_mset(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() || params.len() % 2 != 0 {
        return Err("ERR wrong number of arguments for 'mset' command".to_string());
    }

    let mut key_values: Vec<(String, String)> = Vec::new();
    for pair in params.chunks(2) {
        let tuple = (pair[0].to_string(), pair[1].to_string());
        key_values.push(tuple);
    }
    Ok(Command::Mset { key_values })
}

/// Generador de comando Command::Strlen
fn generate_strlen(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 1 {
        return Err("ERR wrong number of arguments for 'strlen' command".to_string());
    }

    let key = params[0].clone();
    Ok(Command::Strlen { key })
}

/// Generador de comando Command::Dbsize
fn generate_dbsize(params: Vec<String>) -> Result<Command, String> {
    if !params.is_empty() {
        return Err("ERR wrong number of arguments for 'dbsize' command".to_string());
    }

    Ok(Command::Dbsize)
}

/// Generador de comando Command::Lindex
fn generate_lindex(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR wrong number of arguments for 'lindex' command".to_string());
    }

    let key = params[0].clone();
    let index: Result<i32, _> = params[1].to_string().parse();

    if index.is_err() {
        return Err("ERR value is not an integer or out of range".to_string());
    }

    let index = index.unwrap();
    Ok(Command::Lindex { key, index })
}

/// Generador de comando Command::Llen
fn generate_llen(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'llen' command".to_string());
    }

    let key = params[0].to_string();
    Ok(Command::Llen { key })
}

/// Generador de comando Command::Lpop
fn generate_lpop(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() || params.len() > 2 {
        return Err("ERR wrong number of arguments for 'lpop' command".to_string());
    }

    let mut count: usize = 0;
    if params.len() == 2 {
        let parse_count: Result<usize, _> = params[1].to_string().parse();

        if parse_count.is_err() {
            return Err("ERR value is not an integer or out of range".to_string());
        }

        count = parse_count.unwrap();
    }

    let key = params[0].to_string();
    Ok(Command::Lpop { key, count })
}

/// Generador de comando Command::Lrange
fn generate_lrange(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 3 {
        return Err("ERR wrong number of arguments for 'lrange' command".to_string());
    }

    let parse_begin: Result<i32, _> = params[1].to_string().parse();
    if parse_begin.is_err() {
        return Err("ERR value is not an integer or out of range".to_string());
    }

    let begin = parse_begin.unwrap();

    let parse_end: Result<i32, _> = params[2].to_string().parse();
    if parse_end.is_err() {
        return Err("ERR value is not an integer or out of range".to_string());
    }

    let end = parse_end.unwrap();
    let key = params[0].to_string();

    Ok(Command::Lrange { key, begin, end })
}

/// Generador de comando Command::Lrem
fn generate_lrem(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 3 {
        return Err("ERR wrong number of arguments for 'lrem' command".to_string());
    }

    let key = params[0].clone();
    let count: Result<i32, _> = params[1].clone().parse();

    if count.is_err() {
        return Err("ERR value is not an integer or out of range".to_string());
    }

    let element = params[2].clone();
    let count = count.unwrap();

    Ok(Command::Lrem {
        key,
        count,
        element,
    })
}

/// Generador de comando Command::Lset
fn generate_lset(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 3 {
        return Err("ERR wrong number of arguments for 'lset' command".to_string());
    }

    let key = params[0].clone();
    let index: Result<i32, _> = params[1].clone().parse();

    if index.is_err() {
        return Err("ERR value is not an integer or out of range".to_string());
    }

    let element = params[2].clone();
    let index = index.unwrap();

    Ok(Command::Lset {
        key,
        index,
        element,
    })
}

/// Generador de comando Command::Rpop
fn generate_rpop(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() || params.len() > 2 {
        return Err("ERR wrong number of arguments for 'rpop' command".to_string());
    }

    let mut count: usize = 0;
    if params.len() == 2 {
        let parse_count: Result<usize, _> = params[1].to_string().parse();

        if parse_count.is_err() {
            return Err("ERR value is not an integer or out of range".to_string());
        }

        count = parse_count.unwrap();
    }

    let key = params[0].to_string();
    Ok(Command::Rpop { key, count })
}

/// Generador de comando Command::Lpush
fn generate_lpush(params: Vec<String>) -> Result<Command, String> {
    if params.len() <= 1 {
        return Err("ERR wrong number of arguments for 'lpush' command".to_string());
    }

    let key = params[0].clone();
    let values = Vec::from(params.get(1..).unwrap());

    Ok(Command::Lpush { key, value: values })
}

/// Generador de comando Command::Lpushx
fn generate_lpushx(params: Vec<String>) -> Result<Command, String> {
    if params.len() <= 1 {
        return Err("ERR wrong number of arguments for 'lpushx' command".to_string());
    }

    let key = params[0].clone();
    let values = Vec::from(params.get(1..).unwrap());

    Ok(Command::Lpushx { key, value: values })
}

/// Generador de comando Command::Rpush
fn generate_rpush(params: Vec<String>) -> Result<Command, String> {
    if params.len() <= 1 {
        return Err("ERR wrong number of arguments for 'rpush' command".to_string());
    }

    let key = params[0].clone();
    let values = Vec::from(params.get(1..).unwrap());

    Ok(Command::Rpush { key, value: values })
}

/// Generador de comando Command::Rpushx
fn generate_rpushx(params: Vec<String>) -> Result<Command, String> {
    if params.len() <= 1 {
        return Err("ERR wrong number of arguments for 'rpushx' command".to_string());
    }

    let key = params[0].clone();
    let values = Vec::from(params.get(1..).unwrap());

    Ok(Command::Rpushx { key, value: values })
}

/// Generador de comando Command::Sadd
fn generate_sadd(params: Vec<String>) -> Result<Command, String> {
    if params.len() <= 1 {
        return Err("ERR wrong number of arguments for 'sadd' command".to_string());
    }

    let key = params[0].clone();
    let vector = Vec::from(params.get(1..).unwrap());
    let values = HashSet::from_iter(vector);

    Ok(Command::Sadd { key, values })
}

/// Generador de comando Command::Scard
fn generate_scard(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'scard' command".to_string());
    }
    let key = params[0].clone();
    Ok(Command::Scard { key })
}

/// Generador de comando Command::Sismember
fn generate_sismember(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR wrong number of arguments for 'sismember' command".to_string());
    }

    let key = params[0].clone();
    let value = params[1].clone();
    Ok(Command::Sismember { key, value })
}

/// Generador de comando Command::Srem
fn generate_srem(params: Vec<String>) -> Result<Command, String> {
    if params.len() <= 1 {
        return Err("ERR wrong number of arguments for 'srem' command".to_string());
    }

    let key = params[0].clone();
    let vector = Vec::from(params.get(1..).unwrap());
    let values = HashSet::from_iter(vector);

    Ok(Command::Srem { key, values })
}

/// Generador de comando Command::Smembers
fn generate_smembers(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'smembers' command".to_string());
    }

    let key = params[0].clone();
    Ok(Command::Smembers { key })
}

/// Generador de comando Command::Keys
fn generate_keys(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'keys' command".to_string());
    }
    let pattern = params[0].clone();
    Ok(Command::Keys { pattern })
}

/// Generador de comando Command::Store
fn generate_store(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'store' command".to_string());
    }

    let path = params[0].clone();
    Ok(Command::Store { path })
}

/// Generador de comando Command::Load
fn generate_load(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'load' command".to_string());
    }

    let path = params[0].clone();
    Ok(Command::Load { path })
}

/// Generador de comando Command::Pubsub
fn generate_pubsub(params: Vec<String>) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'pubsub' command".to_string());
    }

    match params[0].clone().to_lowercase().as_str() {
        "channels" => match params.len() {
            1 => Ok(Command::Pubsub {
                param: PubSubParam::Channels,
            }),
            2 => Ok(Command::Pubsub {
                param: PubSubParam::ChannelsWithChannel(params[1].clone()),
            }),
            _ => Err(
                "ERR Unknown subcommand or wrong number of arguments for ".to_string()
                    + params[0].as_str(),
            ),
        },
        "numsub" => match params.len() {
            1 => Ok(Command::Pubsub {
                param: PubSubParam::Numsub,
            }),
            _ => Ok(Command::Pubsub {
                param: PubSubParam::NumsubWithChannels(Vec::from(params.get(1..).unwrap())),
            }),
        },
        _ => Err(
            "ERR Unknown subcommand or wrong number of arguments for ".to_string()
                + params[0].as_str(),
        ),
    }
}

/// Generador de comando Command::Subscribe
fn generate_subscribe(params: Vec<String>, client_id: String) -> Result<Command, String> {
    if params.is_empty() {
        return Err("ERR wrong number of arguments for 'subscribe' command".to_string());
    }

    Ok(Command::Subscribe {
        channels: params,
        client_id,
    })
}

/// Generador de comando Command::Publish
fn generate_publish(params: Vec<String>) -> Result<Command, String> {
    if params.len() != 2 {
        return Err("ERR wrong number of arguments for 'publish' command".to_string());
    }
    let channel = params[0].clone();
    let message = params[1].clone();
    Ok(Command::Publish { channel, message })
}

/// Generador de comando Command::Unsubscribe
fn generate_unsubscribe(params: Vec<String>, client_id: String) -> Command {
    Command::Unsubscribe {
        channels: params,
        client_id,
    }
}

#[allow(unused_imports)]
mod test {
    use crate::entities::command::Command;
    use crate::service::command_generator::generate;
    use core::time::Duration;
    use std::collections::HashSet;
    use std::time::SystemTime;

    #[test]
    fn generate_command_with_params_empty_err() {
        let params = vec![];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_with_command_invalid_err() {
        let params = vec!["metodo".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_with_command_ping() {
        let params = vec!["ping".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Ping => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_with_command_monitor() {
        let params = vec!["monitor".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Monitor => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_with_command_flushdb() {
        let params = vec!["flushdb".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Flushdb => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_copy_without_params_err() {
        let params = vec!["copy".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_copy_with_one_param_err() {
        let params = vec!["copy".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_copy_ok() {
        let params = vec!["copy".to_string(), "key".to_string(), "key1".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _key2 = "key1".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Copy {
                key_origin: _key,
                key_destination: _key2,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_get_without_param_err() {
        let params = vec!["get".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_get_ok() {
        let params = vec!["get".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Get { key: _key } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_getset_without_param_err() {
        let params = vec!["getset".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_getset_with_one_param_err() {
        let params = vec!["getset".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_getset_ok() {
        let params = vec!["getset".to_string(), "key".to_string(), "value".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _value = "value".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Getset {
                key: _key,
                value: _value,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_set_without_param_err() {
        let params = vec!["set".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_set_with_one_param_err() {
        let params = vec!["set".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_set_ok() {
        let params = vec!["set".to_string(), "key".to_string(), "value".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _value = "value".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Set {
                key: _key,
                value: _value,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_del_without_param_err() {
        let params = vec!["del".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_del_ok() {
        let params = vec!["del".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _keys = vec!["key".to_string()];
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Del { keys: _keys } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_mget_without_param_err() {
        let params = vec!["mget".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_mget_ok() {
        let params = vec!["mget".to_string(), "key1".to_string(), "key2".to_string()];
        let result = generate(params, "client-test".to_string());

        let _keys = vec!["key1".to_string(), "key2".to_string()];
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Mget { keys: _keys } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_mset_without_param_err() {
        let params = vec!["mset".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_mset_with_missing_value_err() {
        let params = vec![
            "mset".to_string(),
            "key1".to_string(),
            "value1".to_string(),
            "key2".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_mset_ok() {
        let params = vec![
            "mset".to_string(),
            "key1".to_string(),
            "value1".to_string(),
            "key2".to_string(),
            "value2".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        let _pairs = vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ];
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Mset { key_values: _pairs } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_strlen_without_param_err() {
        let params = vec!["strlen".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_strlen_ok() {
        let params = vec!["strlen".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Strlen { key: _key } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_exists_without_param_err() {
        let params = vec!["exists".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_exists_ok() {
        let params = vec!["exists".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _keys = vec!["key".to_string()];
        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Exists { keys: _keys } => true,
            _ => false,
        });

        let params = vec!["exists".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(match result.unwrap() {
            Command::Ping => false,
            _ => true,
        });
    }

    #[test]
    fn generate_command_rename_without_param_err() {
        let params = vec!["rename".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_rename_ok() {
        let params = vec!["rename".to_string(), "key1".to_string(), "key2".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key_origin = "key1".to_string();
        let _key_destination = "key2".to_string();

        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Rename {
                key_origin: _key_origin,
                key_destination: _key_destination,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_expire_without_param_err() {
        let params = vec!["expire".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn generate_command_expire_with_fractional_time_err() {
        let params = vec!["expire".to_string(), "key".to_string(), "10.5".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn generate_command_expire_ok() {
        let params = vec!["expire".to_string(), "key".to_string(), "1".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _ttl = Duration::from_secs(1);

        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Expire {
                key: _key,
                ttl: _ttl,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_expireat_without_param_err() {
        let params = vec!["expireat".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn generate_command_expireat_with_fractional_time_err() {
        let params = vec![
            "expireat".to_string(),
            "key".to_string(),
            "10.5".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn generate_command_expireat_ok() {
        let params = vec!["expireat".to_string(), "key".to_string(), "1".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _ttl = SystemTime::UNIX_EPOCH + Duration::from_secs(1);

        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Expireat {
                key: _key,
                ttl: _ttl,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_persist_without_param_err() {
        let params = vec!["persist".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_persist_ok() {
        let params = vec!["persist".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Persist { key: _key } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_sort_without_param_err() {
        let params = vec!["sort".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_sort_ok() {
        let params = vec!["sort".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Sort { key: _key } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_touch_without_param_err() {
        let params = vec!["touch".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_touch_ok() {
        let params = vec!["touch".to_string(), "key1".to_string(), "key2".to_string()];
        let result = generate(params, "client-test".to_string());

        let _keys = vec!["key1".to_string(), "key2".to_string()];
        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Touch { keys: _keys } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_ttl_without_param_err() {
        let params = vec!["ttl".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_ttl_ok() {
        let params = vec!["ttl".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Ttl { key: _key } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_type_without_param_err() {
        let params = vec!["type".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_type_ok() {
        let params = vec!["type".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();

        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Type { key: _key } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_incrby_without_param_err() {
        let params = vec!["incrby".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["incrby".to_string(), "key".to_string(), "hola".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_incrby_ok() {
        let params = vec!["incrby".to_string(), "key1".to_string(), "1".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key1".to_string();

        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Incrby {
                key: _key,
                increment: 1,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_decrby_without_param_err() {
        let params = vec!["decrby".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["decrby".to_string(), "key".to_string(), "hola".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_decrby_ok() {
        let params = vec!["decrby".to_string(), "key1".to_string(), "1".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key1".to_string();

        assert!(result.is_ok());

        assert!(match result.unwrap() {
            Command::Decrby {
                key: _key,
                decrement: 1,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_getdel_without_param_err() {
        let params = vec!["getdel".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_getdel_ok() {
        let params = vec!["getdel".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Getdel { key: _key } => true,
            _ => false,
        });

        let params = vec!["getdel".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(match result.unwrap() {
            Command::Ping => false,
            _ => true,
        });
    }

    #[test]
    fn generate_command_append_without_param_err() {
        let params = vec!["append".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_append_ok() {
        let params = vec!["append".to_string(), "key".to_string(), "Value".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _value = "Value".to_string();

        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Append {
                key: _key,
                value: _value,
            } => true,
            _ => false,
        });

        let params = vec!["append".to_string(), "key".to_string(), "Value".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(match result.unwrap() {
            Command::Ping => false,
            _ => true,
        });
    }

    #[test]
    fn generate_command_with_command_dbsize() {
        let params = vec!["dbsize".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Dbsize => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_lindex_incorrect_params_err() {
        let params = vec!["lindex".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["lindex".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["lindex".to_string(), "key".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec![
            "lindex".to_string(),
            "key".to_string(),
            "1".to_string(),
            "value".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn generate_command_lindex_ok() {
        let params = vec!["lindex".to_string(), "key".to_string(), "1".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _index = 1;
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Lindex {
                key: _key,
                index: _index,
            } => true,
            _ => false,
        });

        let params = vec!["lindex".to_string(), "key".to_string(), "-1".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _index = -1;
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Lindex {
                key: _key,
                index: _index,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_llen_without_param_err() {
        let params = vec!["llen".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_llen_ok() {
        let params = vec!["llen".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Llen { key: _key } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_lpop_without_param_err() {
        let params = vec!["lpop".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_lpop_without_param_count_not_u32_err() {
        let params = vec!["lpop".to_string(), "key".to_string(), "value".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_lpop_ok() {
        let params = vec!["lpop".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Lpop {
                key: _key,
                count: 0,
            } => true,
            _ => false,
        });

        let params = vec!["lpop".to_string(), "key".to_string(), "3".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Lpop {
                key: _key,
                count: 3,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_lrange_bad_params_err() {
        let params = vec!["lrange".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec![
            "lrange".to_string(),
            "key".to_string(),
            "a".to_string(),
            "1".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec![
            "lrange".to_string(),
            "key".to_string(),
            "1".to_string(),
            "a".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec![
            "lrange".to_string(),
            "key".to_string(),
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
        ];
        let result = generate(params, "client-test".to_string());
        assert!(result.is_err())
    }

    #[test]
    fn generate_command_lrange_ok() {
        let params = vec![
            "lrange".to_string(),
            "key".to_string(),
            "0".to_string(),
            "-1".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Lrange {
                key: _key,
                begin: 0,
                end: -1,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_lrem_err() {
        let params = vec![
            "lrem".to_string(),
            "key".to_string(),
            "-1".to_string(),
            "element".to_string(),
            "element".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["lrem".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec![
            "lrem".to_string(),
            "key".to_string(),
            "a".to_string(),
            "element".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn generate_command_lrem_ok() {
        let params = vec![
            "lrem".to_string(),
            "key".to_string(),
            "0".to_string(),
            "element".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _element = "element".to_string();

        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Lrem {
                key: _key,
                count: 0,
                element: _element,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_lset_err() {
        let params = vec![
            "lset".to_string(),
            "key".to_string(),
            "-1".to_string(),
            "element".to_string(),
            "element".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["lset".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec![
            "lset".to_string(),
            "key".to_string(),
            "a".to_string(),
            "element".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn generate_command_lset_ok() {
        let params = vec![
            "lset".to_string(),
            "key".to_string(),
            "1".to_string(),
            "Hola".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _index = "1".to_string();
        let _element = "Hola".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Lset {
                key: _key,
                index: _index,
                element: _element,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_rpop_without_param_err() {
        let params = vec!["rpop".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_rpop_without_param_count_not_u32_err() {
        let params = vec!["rpop".to_string(), "key".to_string(), "value".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_rpop_ok() {
        let params = vec!["rpop".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Rpop {
                key: _key,
                count: 0,
            } => true,
            _ => false,
        });

        let params = vec!["rpop".to_string(), "key".to_string(), "3".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Rpop {
                key: _key,
                count: 3,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_lpush_incorrect_params_err() {
        let params = vec!["lpush".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["lpush".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_lpush_ok() {
        let params = vec!["lpush".to_string(), "key".to_string(), "value".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _value = vec!["value".to_string()];
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Lpush {
                key: _key,
                value: _value,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_lpushx_incorrect_params_err() {
        let params = vec!["lpushx".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["lpushx".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_rpush_incorrect_params_err() {
        let params = vec!["rpush".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["rpush".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_rpush_ok() {
        let params = vec!["rpush".to_string(), "key".to_string(), "value".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _value = vec!["value".to_string()];
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Rpush {
                key: _key,
                value: _value,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_rpushx_incorrect_params_err() {
        let params = vec!["rpushx".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["rpushx".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_sadd_incorrect_params_err() {
        let params = vec!["sadd".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["sadd".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_lpushx_ok() {
        let params = vec!["lpushx".to_string(), "key".to_string(), "value".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _value = vec!["value".to_string()];
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Lpushx {
                key: _key,
                value: _value,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_sadd_ok() {
        let params = vec!["sadd".to_string(), "key".to_string(), "value".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let mut _values = HashSet::new();
        _values.insert("value1".to_string());
        _values.insert("value2".to_string());
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Sadd {
                key: _key,
                values: _values,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_scard_without_param_err() {
        let params = vec!["scard".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_scard_ok() {
        let params = vec!["scard".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Scard { key: _key } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_sismember_without_param_err() {
        let params = vec!["sismember".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_sismember_ok() {
        let params = vec![
            "sismember".to_string(),
            "key".to_string(),
            "value".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let _value = "value".to_string();

        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Sismember {
                key: _key,
                value: _value,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_srem_incorrect_params_err() {
        let params = vec!["srem".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err());

        let params = vec!["srem".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_srem_ok() {
        let params = vec!["srem".to_string(), "key".to_string(), "value".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        let mut _values = HashSet::new();
        _values.insert("value1".to_string());
        _values.insert("value2".to_string());
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Srem {
                key: _key,
                values: _values,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_smembers_without_param_err() {
        let params = vec!["smembers".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_smembers_ok() {
        let params = vec!["smembers".to_string(), "key".to_string()];
        let result = generate(params, "client-test".to_string());

        let _key = "key".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Smembers { key: _key } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_keys_ok() {
        let params = vec!["keys".to_string(), "/*".to_string()];
        let result = generate(params, "client-test".to_string());

        let _pattern = "/*".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Keys { pattern: _pattern } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_store_without_param_err() {
        let params = vec!["store".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_store_ok() {
        let params = vec!["store".to_string(), "/store.file".to_string()];
        let result = generate(params, "client-test".to_string());

        let _path = "/store.file".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Store { path: _path } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_config_set_without_param_err() {
        let params = vec!["config".to_string(), "set".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_config_set_ok() {
        let params = vec![
            "config".to_string(),
            "set".to_string(),
            "verbose".to_string(),
            "1".to_string(),
        ];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_ok());
        let _parameter = "verbose".to_string();
        let _value = "1".to_string();
        assert!(match result.unwrap() {
            Command::ConfigSet {
                parameter: _parameter,
                value: _value,
            } => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_config_get_ok() {
        let params = vec!["config".to_string(), "get".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::ConfigGet => true,
            _ => false,
        });
    }

    #[test]
    fn generate_command_load_without_param_err() {
        let params = vec!["load".to_string()];
        let result = generate(params, "client-test".to_string());

        assert!(result.is_err())
    }

    #[test]
    fn generate_command_load_ok() {
        let params = vec!["load".to_string(), "/store.file".to_string()];
        let result = generate(params, "client-test".to_string());

        let _path = "/store.file".to_string();
        assert!(result.is_ok());
        assert!(match result.unwrap() {
            Command::Load { path: _path } => true,
            _ => false,
        });
    }
}

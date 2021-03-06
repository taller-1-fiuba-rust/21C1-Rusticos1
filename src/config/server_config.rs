use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

/// Struct usado para representar la configuración posible de nuestra base de datos Redis.
#[derive(Debug)]
pub struct Config {
    /// verbose: Representa si el server debe imprimir sus transacciones por consola en tiempo de
    /// ejecución.
    verbose: u8,
    /// port: Indica el puerto en el cual el servidor estará escuchando peticiones.
    port: u16,
    /// timeout: un valor entero indicando cu ́antos segundos esperar a que un cliente envíe un
    /// comando antes de cerrar la conexión. Si el valor es 0 se deshabilita el timeout.
    timeout: u64,
    /// dbfilename: un string indicando el nombre del archivo en el cual se persistirán los datos
    /// almacenados.
    dbfilename: String,
    /// logfile: un string indicando el nombre del archivo en el cual se grabara el log
    logfile: String,
    /// loglevel: indica el nivel de log a implementar en el server [error:1, info:2, debug:3]
    loglevel: u8,
    /// configfile: guarda en la configuración la ruta del archivo de configuración usado.
    configfile: String,
}

#[allow(dead_code)]
impl Config {
    /// Este metodo permite generar una configuración con valores por defecto.
    pub fn new() -> Config {
        Config {
            verbose: 0,
            port: 8080,
            timeout: 0,
            dbfilename: "dump.rdb".to_string(),
            logfile: "log.log".to_string(),
            loglevel: 3,
            configfile: "file.conf".to_string(),
        }
    }

    /// Este metodo permite generar una configuración con valores definidos en un archivo de conf.
    pub fn new_from_file(path: String) -> Result<Config, io::Error> {
        let mut config = Config::new();
        config.set_configfile(path.clone());
        let path = Path::new(&path);
        let file = File::open(path)?;
        let content = BufReader::new(&file);

        for line in content.lines() {
            // Remuevo espacios al principio y al final de la línea.
            let line = line?;
            let line = line.trim();

            // Verifico si la línea es valida; eg: comentarios, linea en blanco, etc.
            if is_invalid_line(line) {
                continue;
            }

            // Separo el attributo de la config con el valor de la config.
            let splited: Vec<&str> = line.split_whitespace().collect();
            let name = splited.first().unwrap_or(&"");
            let tokens = splited.get(1..).unwrap_or(&[""]);

            let parameters = Config::clean_and_parse_lines(tokens);
            let param = parameters[0].clone();

            // Seteo los valores de la configuración∫
            match name.to_lowercase().as_str() {
                "verbose" => config.set_verbose(param),
                "port" => config.set_port(param),
                "timeout" => config.set_timeout(param),
                "dbfilename" => config.set_dbfilename(param),
                "logfile" => config.set_logfile(param),
                "loglevel" => config.set_loglevel(param),
                _ => (),
            }
        }

        Ok(config)
    }

    fn clean_and_parse_lines(tokens: &[&str]) -> Vec<String> {
        // Remuevo si hay un signo =
        let tokens = tokens.iter().filter(|t| !t.starts_with('='));
        // Remuevo si hay comentarios al final de los params
        let tokens = tokens.take_while(|t| !t.starts_with('#') && !t.starts_with(';'));

        // Concat back the parameters into one string to split for separated parameters
        let mut parameters = String::new();
        tokens.for_each(|t| {
            parameters.push_str(t);
            parameters.push(' ');
        });
        // Splits the parameters and trims
        let parameters = parameters.split(',').map(|s| s.trim());
        // Converts them from Vec<&str> into Vec<String>
        let parameters: Vec<String> = parameters.map(|s| s.to_string()).collect();
        parameters
    }

    pub fn set_verbose(&mut self, verbose: String) {
        let val = verbose.parse::<u8>();
        if let Ok(value) = val {
            self.verbose = value
        }
    }

    fn set_port(&mut self, port: String) {
        let val = port.parse::<u16>();
        if let Ok(value) = val {
            self.port = value
        }
    }

    fn set_timeout(&mut self, timeout: String) {
        let val = timeout.parse::<u64>();
        if let Ok(value) = val {
            self.timeout = value
        }
    }

    pub fn set_dbfilename(&mut self, dbfilename: String) {
        self.dbfilename = dbfilename;
    }

    pub fn set_logfile(&mut self, logfile: String) {
        self.logfile = logfile;
    }

    fn set_configfile(&mut self, configfile: String) {
        self.configfile = configfile;
    }

    fn set_loglevel(&mut self, loglevel: String) {
        match loglevel.to_lowercase().as_str() {
            "error" => self.loglevel = 1,
            "info" => self.loglevel = 2,
            _ => self.loglevel = 3,
        }
    }

    pub fn get_port(&self) -> String {
        self.port.to_string()
    }

    pub fn get_verbose(&self) -> String {
        self.verbose.to_string()
    }

    pub fn get_timeout(&self) -> u64 {
        self.timeout
    }

    pub fn get_dbfilename(&self) -> String {
        self.dbfilename.to_string()
    }

    pub fn get_logfile(&self) -> String {
        self.logfile.to_string()
    }

    pub fn get_configfile(&self) -> String {
        self.configfile.to_string()
    }

    pub fn get_loglevel(&self) -> u8 {
        self.loglevel
    }
}

fn is_invalid_line(line: &str) -> bool {
    line.starts_with('#') || line.starts_with(';') || line.is_empty()
}

#[allow(unused_imports)]
mod test {
    use crate::config::server_config::{is_invalid_line, Config};
    use crate::entities::log_level::LogLevel;
    use std::iter::FromIterator;

    #[test]
    fn check_default_config_values() {
        let config = Config::new();
        assert_eq!("0", config.get_verbose());
        assert_eq!("8080", config.get_port());
        assert_eq!(0, config.get_timeout());
        assert_eq!("dump.rdb".to_string(), config.get_dbfilename());
        assert_eq!("log.log".to_string(), config.get_logfile());
        assert_eq!(3, config.loglevel);
    }

    #[test]
    fn clean_and_parse_lines() {
        let line: &str = "dbnombre.rbd # Listado de elementos comentados";
        let splited = Vec::from_iter(line.split_whitespace());
        let vec = splited.get(0..).unwrap();
        let params = Config::clean_and_parse_lines(vec);

        assert_eq!(1, params.len())
    }

    #[test]
    fn check_line_is_valid_false() {
        let line: &str = "#esta línea no es valida";
        assert!(is_invalid_line(line));

        let line: &str = ";esta línea no es valida";
        assert!(is_invalid_line(line));

        let line: &str = "esta línea es valida";
        assert!(!is_invalid_line(line))
    }
}

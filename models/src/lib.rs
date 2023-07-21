use uuid::Uuid;
use serde::{Serialize,Deserialize};
use chrono::NaiveDate;

#[derive(Debug,Serialize,Deserialize)]
pub enum Column {
    Uuid(Option<Uuid>),
    String(Option<String>),
    Integer(Option<i64>),
    Float(Option<f64>),
    Date(Option<NaiveDate>),
}

#[derive(Debug,Serialize,Deserialize)]
pub struct Row{
    pub sheet_id : Uuid,
    pub columns : Vec<Column>,
}

#[derive(Debug,Serialize,Deserialize)]
pub struct Sheet{
    pub id : Uuid,
    pub sheet_name : String,
    pub sheet_type : String,
    pub insert_date : NaiveDate,
    pub rows : Vec<Row>,
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub enum ColumnConfig {
    Uuid(String),
    String(String),
    Integer(String),
    Float(String),
    Date(String),
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub enum Opetation {
    Multiply(ColumnConfig,ColumnConfig),
    MultiplyO(ColumnConfig,Box<Opetation>),
    Add(ColumnConfig,ColumnConfig),
    AddO(ColumnConfig,Box<Opetation>),
    Minus(ColumnConfig,ColumnConfig),
    MinusO(Box<Opetation>,ColumnConfig),
    OMinus(ColumnConfig,Box<Opetation>),
    Divide(ColumnConfig,ColumnConfig),
    DivideO(ColumnConfig,Box<Opetation>),
    ODivide(Box<Opetation>,ColumnConfig),
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub enum ConfigValue{
    Basic(ColumnConfig),
    Calculated(Opetation)
}

#[derive(Debug,Serialize,Deserialize)]
pub struct SheetConfig{
    pub sheet_type_name : String,
    pub row : Vec<ConfigValue>,
}

#[derive(Debug,Serialize,Deserialize)]
pub struct Config{
    pub sheets : Vec<SheetConfig>
}

use std::fs::File;
use std::io::Write;

pub fn get_config_example(){
    let a = Config{
	sheets : vec![
	    SheetConfig{
		sheet_type_name: String::from("مبيعات"),
		row :  vec![
		    ConfigValue::Basic(ColumnConfig::Uuid("id".to_string())),
		    ConfigValue::Basic(ColumnConfig::String("name".to_string())),
		    ConfigValue::Basic(ColumnConfig::String("desc".to_string())),
		    ConfigValue::Basic(ColumnConfig::Float("tax".to_string())),
		    ConfigValue::Basic(ColumnConfig::Integer("i32".to_string())),
		    ConfigValue::Basic(ColumnConfig::Date("date".to_string())),
		],
	    },
	    SheetConfig{
		sheet_type_name: String::from("مشتريات"),
		row :  vec![
		    ConfigValue::Basic(ColumnConfig::Uuid("id".to_string())),
		    ConfigValue::Basic(ColumnConfig::String("name".to_string())),
		    ConfigValue::Basic(ColumnConfig::Float("tax".to_string())),
		    ConfigValue::Basic(ColumnConfig::Integer("i32".to_string())),
		    ConfigValue::Calculated(
			Opetation::AddO(
			    ColumnConfig::Float("tax".to_string()),
			    Box::new(Opetation::Multiply(ColumnConfig::Float("tax".to_string()),
						ColumnConfig::Integer("i32".to_string())))))
		],
	    },
	]
    };

    let b = serde_json::to_string(&a).unwrap_or_default();
    
    let mut file = File::create("output.json").unwrap();

    file.write_all(b.as_bytes()).unwrap();
}

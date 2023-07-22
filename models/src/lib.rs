use uuid::Uuid;
use serde::{Serialize,Deserialize};
use chrono::NaiveDate;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SheetShearchParams {
    pub offset: i64,
    pub begin: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
    pub sheet_name: Option<String>,
    pub sheet_type_name: String,
}

#[derive(Debug,Serialize,Deserialize,Clone,Default)]
pub struct Name{
    pub id : Uuid,
    pub the_name : String,
}

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
    pub sheet_type_name : String,
    pub insert_date : NaiveDate,
    pub rows : Vec<Row>,
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct ColumnProps{
    pub header: String,
    pub is_completable : bool,
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub enum ColumnConfig {
    Uuid(ColumnProps),
    String(ColumnProps),
    Integer(ColumnProps),
    Float(ColumnProps),
    Date(ColumnProps),
}

type OpetationValue = (String,String);
type OpetationOValue = (Box<Opetation>,String);
type OpetationValueO = (String,Box<Opetation>);

#[derive(Debug,Serialize,Deserialize,Clone)]
pub enum Opetation {
    Multiply(OpetationValue),
    Add(OpetationValue),
    Minus(OpetationValue),
    Divide(OpetationValue),
    OMultiply(OpetationOValue),
    OAdd(OpetationOValue),
    OMinus(OpetationOValue),
    ODivide(OpetationOValue),
    MultiplyO(OpetationValueO),
    AddO(OpetationValueO),
    MinusO(OpetationValueO),
    DivideO(OpetationValueO),
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
    let fcp = |header| ColumnProps{
	header,
	is_completable: false
    };
    let tcp = |header| ColumnProps{
	header,
	is_completable: true
    };
    let a = Config{
	sheets : vec![
	    SheetConfig{
		sheet_type_name: String::from("مبيعات"),
		row :  vec![
		    ConfigValue::Basic(ColumnConfig::Uuid(fcp("id".to_string()))),
		    ConfigValue::Basic(ColumnConfig::String(tcp("name".to_string()))),
		    ConfigValue::Basic(ColumnConfig::String(tcp("desc".to_string()))),
		    ConfigValue::Basic(ColumnConfig::Float(fcp("tax".to_string()))),
		    ConfigValue::Basic(ColumnConfig::Integer(fcp("i32".to_string()))),
		    ConfigValue::Basic(ColumnConfig::Date(fcp("date".to_string()))),
		],
	    },
	    SheetConfig{
		sheet_type_name: String::from("مشتريات"),
		row :  vec![
		    ConfigValue::Basic(ColumnConfig::Uuid(fcp("id".to_string()))),
		    ConfigValue::Basic(ColumnConfig::String(tcp("name".to_string()))),
		    ConfigValue::Basic(ColumnConfig::Float(fcp("tax".to_string()))),
		    ConfigValue::Basic(ColumnConfig::Integer(fcp("i32".to_string()))),
		    ConfigValue::Calculated(
			Opetation::AddO((
			    "tax".to_string(),
			    Box::new(Opetation::Multiply((
				"tax".to_string(),
				"i32".to_string()))))))
		],
	    },
	]
    };

    let b = serde_json::to_string(&a).unwrap_or_default();
    
    let mut file = File::create("output.json").unwrap();

    file.write_all(b.as_bytes()).unwrap();
}

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::MAIN_SEPARATOR;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SearchSheetParams {
    pub offset: i64,
    pub begin: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
    pub sheet_name: Option<String>,
    pub sheet_type_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Name {
    pub id: Uuid,
    pub the_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ColumnValue {
    String(Option<String>),
    Float(f64),
    Date(Option<NaiveDate>),
}

impl ToString for ColumnValue {
    fn to_string(&self) -> String {
        match self {
            Self::String(Some(v)) => v.to_owned(),
            Self::Float(v) => format!("{:.2}", v),
            Self::Date(Some(v)) => v.to_string(),
            _ => String::from(""),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Column {
    pub is_basic: bool,
    pub value: ColumnValue,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Row {
    pub id: Uuid,
    pub columns: HashMap<String, Column>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct Sheet {
    pub id: Uuid,
    pub sheet_name: String,
    pub type_name: String,
    pub insert_date: NaiveDate,
    pub rows: Vec<Row>,
}

pub trait HeaderGetter {
    fn get_header(self) -> String;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ColumnProps {
    pub header: String,
    pub is_completable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ColumnConfig {
    String(ColumnProps),
    Float(ColumnProps),
    Date(ColumnProps),
}

impl HeaderGetter for ColumnConfig {
    fn get_header(self) -> String {
        match self {
            Self::String(prop) => prop.header,
            Self::Float(prop) => prop.header,
            Self::Date(prop) => prop.header,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ValueType {
    Const(f64),
    Variable(String),
}

type OperationValue = (ValueType, ValueType);
type OperationOValue = (Box<Operation>, ValueType);
type OperationValueO = (ValueType, Box<Operation>);

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Operation {
    Multiply(OperationValue),
    Add(OperationValue),
    Minus(OperationValue),
    Divide(OperationValue),
    OMultiply(OperationOValue),
    OAdd(OperationOValue),
    OMinus(OperationOValue),
    ODivide(OperationOValue),
    MultiplyO(OperationValueO),
    AddO(OperationValueO),
    MinusO(OperationValueO),
    DivideO(OperationValueO),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct OperationConfig {
    pub header: String,
    pub value: Operation,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConfigValue {
    Basic(ColumnConfig),
    Calculated(OperationConfig),
}

impl HeaderGetter for ConfigValue {
    fn get_header(self) -> String {
        match self {
            Self::Basic(cv) => cv.get_header(),
            Self::Calculated(cv) => cv.header,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportConfig {
    pub unique : Vec<JsonConfig>,
    pub repeated_location : JsonConfig,
    pub repeated : Vec<JsonConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum JsonConfig {
    Value(String,String),
    Object(String,Box<JsonConfig>),
    Location(Box<JsonConfig>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SheetConfig {
    pub sheet_type_name: String,
    pub importing : ImportConfig,
    pub row: Vec<ConfigValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub import_path : String,
    pub sheets: Vec<SheetConfig>,
}

use std::fs::File;
use std::io::Write;

pub fn get_config_example() {
    let fcp = |header| ColumnProps {
        header,
        is_completable: false,
    };
    let tcp = |header| ColumnProps {
        header,
        is_completable: true,
    };
    let a = Config {
	import_path : format!("{}{}Downloads{}",
			      dirs::home_dir().unwrap_or_default().display(),
			      MAIN_SEPARATOR,
			      MAIN_SEPARATOR,
	),
        sheets: vec![
            SheetConfig {
                sheet_type_name: String::from("مبيعات"),
		importing : ImportConfig{
		    unique : vec![],
		    repeated_location : JsonConfig::Location(
				       Box::new(JsonConfig::Value(
					   "document".to_string(),
					   "invoiceLines".to_string()))),
		    repeated : vec![]
		},
                row: vec![
                    ConfigValue::Basic(ColumnConfig::Float(fcp("رقم الفاتورة".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Date(fcp("التاريخ".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("رقم التسجيل الضريبي".to_string()))),
                    ConfigValue::Basic(ColumnConfig::String(tcp("اسم العميل".to_string()))),
                    ConfigValue::Basic(ColumnConfig::String(tcp("تبع".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("القيمة".to_string()))),
                    ConfigValue::Calculated(OperationConfig {
                        header: "ض.ق.م".to_string(),
                        value: Operation::Multiply((
                            ValueType::Variable("القيمة".to_string()),
                            ValueType::Const(0.14),
                        )),
                    }),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("الخصم".to_string()))),
                    ConfigValue::Calculated(OperationConfig {
                        header: "الاجمالي".to_string(),
                        value: Operation::AddO((
                            ValueType::Variable("القيمة".to_string()),
                            Box::new(Operation::OMinus((
                                Box::new(Operation::Multiply((
                                    ValueType::Variable("القيمة".to_string()),
                                    ValueType::Const(0.14),
                                ))),
                                ValueType::Variable("الخصم".to_string()),
                            ))),
                        )),
                    }),
                ],
            },
            SheetConfig {
                sheet_type_name: String::from("مشتريات"),
		importing : ImportConfig{
		    unique : vec![],
		    repeated_location : JsonConfig::Location(
				       Box::new(JsonConfig::Value(
					   "document".to_string(),
					   "invoiceLines".to_string()))),
		    repeated : vec![]
		},
                row: vec![
                    ConfigValue::Basic(ColumnConfig::Float(fcp("رقم الفاتورة".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Date(fcp("التاريخ".to_string()))),
                    ConfigValue::Basic(ColumnConfig::String(tcp("بيان".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("رقم الاصناف".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("السعر".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("العدد".to_string()))),
                    ConfigValue::Calculated(OperationConfig {
                        header: "الاجمالي".to_string(),
                        value: Operation::Multiply((
                            ValueType::Variable("السعر".to_string()),
                            ValueType::Variable("العدد".to_string()),
			)),
                    }),
                ],
            },
            SheetConfig {
                sheet_type_name: String::from("كارت صنف"),
		importing : ImportConfig{
		    unique : vec![
			JsonConfig::Object("رقم الفاتورة".to_string(),
				       Box::new(JsonConfig::Value(
					   "document".to_string(),
					   "internalID".to_string()))),
			JsonConfig::Object("التاريخ".to_string(), 
				       Box::new(JsonConfig::Value(
					   "document".to_string(),
					   "dateTimeIssued".to_string())))],
		    repeated_location : JsonConfig::Location(
				       Box::new(JsonConfig::Value(
					   "document".to_string(),
					   "invoiceLines".to_string()))),
		    repeated : vec![
			JsonConfig::Value("كود الصنف".to_string(), "itemCode".to_string()),
			JsonConfig::Value("اسم الصنف".to_string(), "description".to_string()),
			JsonConfig::Object("السعر".to_string(),
				       Box::new(JsonConfig::Value("unitValue".to_string(),
								  "amountEGP".to_string()))),
			JsonConfig::Value("الكمية".to_string(), "quantity".to_string()),
		]},
                row: vec![
                    ConfigValue::Basic(ColumnConfig::Float(fcp("رقم الفاتورة".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Date(fcp("التاريخ".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("كود الصنف".to_string()))),
                    ConfigValue::Basic(ColumnConfig::String(tcp("اسم الصنف".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("السعر".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("الكمية".to_string()))),
                    ConfigValue::Calculated(OperationConfig {
                        header: "القيمة".to_string(),
                        value: Operation::Multiply((
                            ValueType::Variable("السعر".to_string()),
                            ValueType::Variable("الكمية".to_string()),
			)),
                    }),
                ],
            },
        ],
    };

    let b = serde_json::to_string(&a).unwrap_or_default();

    let mut file = File::create("output.json").unwrap();

    file.write_all(b.as_bytes()).unwrap();
}

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SheetShearchParams {
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

#[derive(Debug, Serialize, Deserialize)]
pub enum Column {
    String(Option<String>),
    Float(f64),
    Date(Option<NaiveDate>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Row {
    pub sheet_id: Uuid,
    pub columns: Vec<Column>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sheet {
    pub id: Uuid,
    pub sheet_name: String,
    pub sheet_type_name: String,
    pub insert_date: NaiveDate,
    pub rows: Vec<Row>,
}

pub trait HeaderGetter {
    fn get_header(self) -> String;
}

#[derive(Debug, Serialize, Deserialize, Clone,PartialEq)]
pub struct ColumnProps {
    pub header: String,
    pub is_completable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone,PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Clone,PartialEq)]
pub enum ValueType {
    Const(f64),
    Variable(String),
}

type OperationValue = (ValueType, ValueType);
type OperationOValue = (Box<Operation>, ValueType);
type OperationValueO = (ValueType, Box<Operation>);

#[derive(Debug, Serialize, Deserialize, Clone,PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Clone,PartialEq)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SheetConfig {
    pub sheet_type_name: String,
    pub row: Vec<ConfigValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
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
        sheets: vec![
            SheetConfig {
                sheet_type_name: String::from("مبيعات"),
                row: vec![
                    ConfigValue::Basic(ColumnConfig::Float(fcp("رقم الفاتورة".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Date(fcp("التاريخ".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp(
                        "رقم التسجيل الضريبي".to_string()
                    ))),
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
                row: vec![
                    ConfigValue::Basic(ColumnConfig::String(tcp("name".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("tax".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("i32".to_string()))),
                ],
            },
        ],
    };

    let b = serde_json::to_string(&a).unwrap_or_default();

    let mut file = File::create("output.json").unwrap();

    file.write_all(b.as_bytes()).unwrap();
}

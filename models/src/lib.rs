use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ColumnId {
    pub sheet_id: Uuid,
    pub row_id: Uuid,
    pub header: String,
}

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

impl Column {
    fn compare(&self, other: &Column) -> Option<Ordering> {
        match (self.value.clone(), other.value.clone()) {
            (ColumnValue::Float(n1), ColumnValue::Float(n2)) => Some(if n1 > n2 {
                Ordering::Greater
            } else if n1 < n2 {
                Ordering::Less
            } else {
                Ordering::Equal
            }),
            (ColumnValue::String(Some(s1)), ColumnValue::String(Some(s2))) => Some(s1.cmp(&s2)),
            (ColumnValue::Date(Some(b1)), ColumnValue::Date(Some(b2))) => Some(b1.cmp(&b2)),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Row {
    pub id: Uuid,
    pub columns: HashMap<String, Column>,
}

pub trait RowsSort {
    fn sort_rows(&mut self, keys: Vec<String>);
}

impl RowsSort for Vec<Row> {
    fn sort_rows(&mut self, keys: Vec<String>) {
        self.sort_by(|row_one, row_two| {
            let mut result = Ordering::Equal;
            for key in keys.iter() {
                let column_one = row_one.columns.get(key);
                let column_two = row_two.columns.get(key);
                if let (Some(column_one), Some(column_two)) = (column_one, column_two) {
                    if let Some(ordering) = column_one.compare(column_two) {
                        if ordering != Ordering::Equal {
                            result = ordering;
                            break;
                        }
                    };
                }
            }
            result
        });
    }
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
    Operation(Box<Operation>),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Operation {
    pub op: OperationKind,
    pub lhs: ValueType,
    pub rhs: ValueType,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum OperationKind {
    Multiply,
    Add,
    Minus,
    Divide,
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
    pub main_entry: Vec<String>,
    pub repeated_entry: Vec<String>,
    pub unique: HashMap<String, Vec<String>>,
    pub repeated: HashMap<String, Vec<String>>,
    pub primary: HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IdentityDiffsOps {
    Sum,
    Prod,
    Max,
    Min,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RowIdentity {
    pub id: Vec<String>,
    pub diff_ops: Vec<(String, IdentityDiffsOps)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SheetConfig {
    pub sheet_type_name: String,
    pub importing: ImportConfig,
    pub row: Vec<ConfigValue>,
    pub row_identity: RowIdentity,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub priorities: HashMap<String, Vec<String>>,
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
        priorities: HashMap::from([
            (
                String::from("مبيعات"),
                vec!["التاريخ".to_string(), "رقم الفاتورة".to_string()],
            ),
            (String::from("مشتريات"), vec!["التاريخ".to_string()]),
            (String::from("كارت صنف"), vec!["التاريخ".to_string()]),
        ]),
        sheets: vec![
            SheetConfig {
                row_identity: RowIdentity {
                    id: vec![
                        "رقم الفاتورة".to_string(),
                        "التاريخ".to_string(),
                        "رقم التسجيل الضريبي".to_string(),
                        "اسم العميل".to_string(),
                    ],
                    diff_ops: vec![("القيمة".to_string(), IdentityDiffsOps::Sum)],
                },
                sheet_type_name: String::from("مبيعات"),
                importing: ImportConfig {
                    main_entry: vec![String::from("document")],
                    repeated_entry: vec!["invoiceLines".to_string()],
                    unique: HashMap::from([
                        ("رقم الفاتورة".to_string(), vec!["internalID".to_string()]),
                        ("التاريخ".to_string(), vec!["dateTimeIssued".to_string()]),
                        (
                            "رقم التسجيل الضريبي".to_string(),
                            vec!["receiver".to_string(), "id".to_string()],
                        ),
                        (
                            "اسم العميل".to_string(),
                            vec!["receiver".to_string(), "name".to_string()],
                        ),
                    ]),
                    repeated: HashMap::from([(
                        "القيمة".to_string(),
                        vec!["unitValue".to_string(), "amountEGP".to_string()],
                    )]),
                    primary: HashMap::from([(
                        "اسم الشركة".to_string(),
                        vec!["issuer".to_string(), "name".to_string()],
                    )]),
                },
                row: vec![
                    ConfigValue::Basic(ColumnConfig::Float(fcp("رقم الفاتورة".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Date(fcp("التاريخ".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("رقم التسجيل الضريبي".to_string()))),
                    ConfigValue::Basic(ColumnConfig::String(tcp("اسم العميل".to_string()))),
                    ConfigValue::Basic(ColumnConfig::String(tcp("تبع".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("القيمة".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("الخصم".to_string()))),
                    ConfigValue::Calculated(OperationConfig {
                        header: "ض.ق.م".to_string(),
                        value: Operation {
                            op: OperationKind::Multiply,
                            lhs: ValueType::Variable("القيمة".to_string()),
                            rhs: ValueType::Const(0.14),
                        },
                    }),
                    ConfigValue::Calculated(OperationConfig {
                        header: "الاجمالي".to_string(),
                        value: Operation {
                            op: OperationKind::Add,
                            lhs: ValueType::Variable("القيمة".to_string()),
                            rhs: ValueType::Operation(Box::new(Operation {
                                op: OperationKind::Minus,
                                lhs: ValueType::Operation(Box::new(Operation {
                                    op: OperationKind::Multiply,
                                    lhs: ValueType::Variable("القيمة".to_string()),
                                    rhs: ValueType::Const(0.14),
                                })),
                                rhs: ValueType::Variable("الخصم".to_string()),
                            })),
                        },
                    }),
                ],
            },
            SheetConfig {
                row_identity: RowIdentity {
                    id: vec![],
                    diff_ops: vec![],
                },
                sheet_type_name: String::from("مشتريات"),
                importing: ImportConfig {
                    main_entry: vec![String::from("document")],
                    repeated_entry: vec!["invoiceLines".to_string()],
                    unique: HashMap::from([
                        ("رقم الفاتورة".to_string(), vec!["internalID".to_string()]),
                        ("التاريخ".to_string(), vec!["dateTimeIssued".to_string()]),
                    ]),
                    repeated: HashMap::from([
                        ("بيان".to_string(), vec!["description".to_string()]),
                        ("العدد".to_string(), vec!["quantity".to_string()]),
                        (
                            "السعر".to_string(),
                            vec!["unitValue".to_string(), "amountEGP".to_string()],
                        ),
                    ]),
                    primary: HashMap::from([(
                        "اسم الشركة".to_string(),
                        vec!["issuer".to_string(), "name".to_string()],
                    )]),
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
                        value: Operation {
                            op: OperationKind::Multiply,
                            lhs: ValueType::Variable("السعر".to_string()),
                            rhs: ValueType::Variable("العدد".to_string()),
                        },
                    }),
                ],
            },
            SheetConfig {
                row_identity: RowIdentity {
                    id: vec![],
                    diff_ops: vec![],
                },
                sheet_type_name: String::from("كارت صنف"),
                importing: ImportConfig {
                    main_entry: vec![String::from("document")],
                    repeated_entry: vec!["invoiceLines".to_string()],
                    unique: HashMap::from([
                        ("رقم الفاتورة".to_string(), vec!["internalID".to_string()]),
                        ("التاريخ".to_string(), vec!["dateTimeIssued".to_string()]),
                    ]),
                    repeated: HashMap::from([
                        ("كود الصنف".to_string(), vec!["itemCode".to_string()]),
                        ("اسم الصنف".to_string(), vec!["description".to_string()]),
                        (
                            "السعر".to_string(),
                            vec!["unitValue".to_string(), "amountEGP".to_string()],
                        ),
                        ("الكمية".to_string(), vec!["quantity".to_string()]),
                    ]),
                    primary: HashMap::from([(
                        "اسم الشركة".to_string(),
                        vec!["issuer".to_string(), "name".to_string()],
                    )]),
                },
                row: vec![
                    ConfigValue::Basic(ColumnConfig::Float(fcp("رقم الفاتورة".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Date(fcp("التاريخ".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("كود الصنف".to_string()))),
                    ConfigValue::Basic(ColumnConfig::String(tcp("اسم الصنف".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("الكمية".to_string()))),
                    ConfigValue::Basic(ColumnConfig::Float(fcp("السعر".to_string()))),
                    ConfigValue::Calculated(OperationConfig {
                        header: "القيمة".to_string(),
                        value: Operation {
                            op: OperationKind::Multiply,
                            lhs: ValueType::Variable("السعر".to_string()),
                            rhs: ValueType::Variable("الكمية".to_string()),
                        },
                    }),
                ],
            },
        ],
    };

    let b = serde_json::to_string(&a).unwrap_or_default();

    let mut file = File::create("output.json").unwrap();

    file.write_all(b.as_bytes()).unwrap();
}

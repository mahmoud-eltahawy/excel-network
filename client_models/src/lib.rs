use ciborium_io::Write;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::rc::Rc;
use std::{collections::HashMap, fs::File, io::Cursor, sync::Arc};

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
    fn get_header(self) -> Rc<str> {
        match self {
            Self::Basic(cv) => cv.get_header(),
            Self::Calculated(cv) => Rc::from(cv.header),
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
    Nth(usize),
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RowIdentity<RC>
where
    RC: Hash + Eq,
{
    pub id: RC,
    pub diff_ops: HashMap<RC, IdentityDiffsOps>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SheetConfig<RC>
where
    RC: Hash + Eq,
{
    pub sheet_type_name: RC,
    pub importing: ImportConfig,
    pub row: Vec<ConfigValue>,
    pub row_identity: RowIdentity<RC>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub priorities: HashMap<Arc<str>, Arc<[Arc<str>]>>,
    pub sheets: Vec<SheetConfig<Arc<str>>>,
}

pub trait HeaderGetter {
    fn get_header(self) -> Rc<str>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ColumnConfig {
    String(ColumnProps),
    Float(ColumnProps),
    Date(ColumnProps),
}

impl HeaderGetter for ColumnConfig {
    fn get_header(self) -> Rc<str> {
        match self {
            Self::String(prop) => Rc::from(prop.header),
            Self::Float(prop) => Rc::from(prop.header),
            Self::Date(prop) => Rc::from(prop.header),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ColumnProps {
    pub header: String,
    pub is_completable: bool,
}

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
                Arc::from("مبيعات"),
                Arc::from(vec![Arc::from("التاريخ"), Arc::from("رقم الفاتورة")]),
            ),
            (Arc::from("مشتريات"), Arc::from(vec![Arc::from("التاريخ")])),
            (Arc::from("كارت صنف"), Arc::from(vec![Arc::from("التاريخ")])),
        ]),
        sheets: vec![
            SheetConfig {
                row_identity: RowIdentity {
                    id: Arc::from("رقم الفاتورة"),
                    diff_ops: HashMap::from([
                        (Arc::from("القيمة"), IdentityDiffsOps::Sum),
                        (Arc::from("رقم الفاتورة"), IdentityDiffsOps::Nth(1)),
                        (Arc::from("التاريخ"), IdentityDiffsOps::Nth(1)),
                        (Arc::from("اسم العميل"), IdentityDiffsOps::Nth(1)),
                        (Arc::from("رقم التسجيل الضريبي"), IdentityDiffsOps::Nth(1)),
                        (Arc::from("تبع"), IdentityDiffsOps::Nth(1)),
                        (Arc::from("الخصم"), IdentityDiffsOps::Sum),
                    ]),
                },
                sheet_type_name: Arc::from("مبيعات"),
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
                    id: Arc::from(""),
                    diff_ops: HashMap::new(),
                },
                sheet_type_name: Arc::from("مشتريات"),
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
                    id: Arc::from(""),
                    diff_ops: HashMap::new(),
                },
                sheet_type_name: Arc::from("كارت صنف"),
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

    let mut buf = vec![];
    let mut file_cbor = File::create("output").unwrap();
    ciborium::ser::into_writer(&a, Cursor::new(&mut buf)).unwrap();

    file_cbor.write_all(&buf).unwrap();

    let v: ciborium::Value = ciborium::de::from_reader(Cursor::new(buf)).unwrap();
    let e: Config = v.deserialized().unwrap();
    dbg!(e);
}

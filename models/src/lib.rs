use chrono::NaiveDate;
use ciborium_io::Write;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::{cmp::Ordering, collections::HashMap, io::Cursor, marker::Sized, rc::Rc};
use uuid::Uuid;

use std::fs::File;

pub trait ToSerial<T>: Sized {
    fn to_serial(self) -> T;
}

impl ToSerial<Arc<str>> for Uuid {
    fn to_serial(self) -> Arc<str> {
        Arc::from(self.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ColumnIdSerial<RC>
where
    RC: Eq + Hash + ToString,
{
    pub sheet_id: Arc<str>,
    pub row_id: Arc<str>,
    pub header: RC,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ColumnId<RC>
where
    RC: Eq + Hash + ToString,
{
    pub sheet_id: Uuid,
    pub row_id: Uuid,
    pub header: RC,
}

impl<T> ToSerial<ColumnIdSerial<T>> for ColumnId<T>
where
    T: Eq + Hash + ToString,
{
    fn to_serial(self) -> ColumnIdSerial<T> {
        let ColumnId {
            sheet_id,
            row_id,
            header,
        } = self;
        let sheet_id = sheet_id.to_serial();
        let row_id = row_id.to_serial();
        ColumnIdSerial {
            sheet_id,
            row_id,
            header,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SearchSheetParams {
    pub offset: i64,
    pub begin: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
    pub sheet_name: Option<String>,
    pub sheet_type_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NameSerial {
    pub id: Arc<str>,
    pub the_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Name {
    pub id: Uuid,
    pub the_name: String,
}

impl ToSerial<NameSerial> for Name {
    fn to_serial(self) -> NameSerial {
        let Name { id, the_name } = self;
        let id = id.to_serial();
        NameSerial { id, the_name }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ColumnValue<RC>
where
    RC: Eq + Hash + ToString,
{
    String(Option<RC>),
    Float(f64),
    Date(Option<NaiveDate>),
}

impl<T> ToString for ColumnValue<T>
where
    T: Eq + Hash + ToString,
{
    fn to_string(&self) -> String {
        match self {
            Self::String(Some(v)) => v.to_string(),
            Self::Float(v) => format!("{:.2}", v),
            Self::Date(Some(v)) => v.to_string(),
            _ => String::from(""),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Column<RC>
where
    RC: Eq + Hash + ToString,
{
    pub is_basic: bool,
    pub value: ColumnValue<RC>,
}

impl<T> Column<T>
where
    T: Eq + Hash + ToString + Ord + Clone,
{
    fn compare(&self, other: &Column<T>) -> Option<Ordering> {
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
pub struct RowSerial<RC>
where
    RC: Eq + Hash + ToString,
{
    pub id: Arc<str>,
    pub columns: HashMap<RC, Column<RC>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Row<RC>
where
    RC: Eq + Hash + ToString,
{
    pub id: Uuid,
    pub columns: HashMap<RC, Column<RC>>,
}

impl ToSerial<RowSerial<Arc<str>>> for Row<Arc<str>> {
    fn to_serial(self) -> RowSerial<Arc<str>> {
        let Row { id, columns } = self;
        let id = id.to_serial();
        RowSerial { id, columns }
    }
}

pub trait RowsSort {
    fn sort_rows(&mut self, keys: Rc<[Rc<str>]>);
}

impl RowsSort for Vec<Row<Rc<str>>> {
    fn sort_rows(&mut self, keys: Rc<[Rc<str>]>) {
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SheetSerial<RC>
where
    RC: Eq + Hash + ToString,
{
    pub id: Arc<str>,
    pub sheet_name: RC,
    pub type_name: RC,
    pub insert_date: NaiveDate,
    pub rows: Vec<RowSerial<Arc<str>>>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct Sheet<RC>
where
    RC: Eq + Hash + ToString,
{
    pub id: Uuid,
    pub sheet_name: RC,
    pub type_name: RC,
    pub insert_date: NaiveDate,
    pub rows: Vec<Row<RC>>,
}

use std::sync::Arc;

impl ToSerial<SheetSerial<Arc<str>>> for Sheet<Arc<str>> {
    fn to_serial(self) -> SheetSerial<Arc<str>> {
        let Sheet {
            id,
            sheet_name,
            type_name,
            insert_date,
            rows,
        } = self;
        let id = id.to_serial();
        let rows = rows
            .into_iter()
            .map(|row| row.to_serial())
            .collect::<Vec<_>>();
        SheetSerial {
            id,
            sheet_name,
            type_name,
            insert_date,
            rows,
        }
    }
}

pub trait HeaderGetter {
    fn get_header(self) -> Rc<str>;
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
    fn get_header(self) -> Rc<str> {
        match self {
            Self::String(prop) => Rc::from(prop.header),
            Self::Float(prop) => Rc::from(prop.header),
            Self::Date(prop) => Rc::from(prop.header),
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

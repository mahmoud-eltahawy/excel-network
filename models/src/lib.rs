use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;
use std::sync::Arc;
use std::{cmp::Ordering, collections::HashMap, marker::Sized, rc::Rc};
use uuid::Uuid;

pub trait ToSerial<T>: Sized {
    fn to_serial(self) -> T;
}

impl ToSerial<Arc<str>> for Uuid {
    fn to_serial(self) -> Arc<str> {
        Arc::from(self.to_string())
    }
}

pub trait IdMarker {}
impl IdMarker for Arc<str> {}
impl IdMarker for Rc<str> {}
impl IdMarker for Uuid {}
pub trait StrMarker {}
impl StrMarker for Arc<str> {}
impl StrMarker for Rc<str> {}
impl StrMarker for String {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ColumnId<I, RC>
where
    RC: Eq + Hash + ToString + StrMarker,
    I: IdMarker,
{
    pub sheet_id: I,
    pub row_id: I,
    pub header: RC,
}

impl<S> ToSerial<ColumnId<Arc<str>, S>> for ColumnId<Uuid, S>
where
    S: Eq + Hash + ToString + StrMarker,
{
    fn to_serial(self) -> ColumnId<Arc<str>, S> {
        let ColumnId {
            sheet_id,
            row_id,
            header,
        } = self;
        let sheet_id = sheet_id.to_serial();
        let row_id = row_id.to_serial();
        ColumnId {
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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Name<I>
where
    I: IdMarker,
{
    pub id: I,
    pub the_name: String,
}

impl ToSerial<Name<Arc<str>>> for Name<Uuid> {
    fn to_serial(self) -> Name<Arc<str>> {
        let Name { id, the_name } = self;
        let id = id.to_serial();
        Name { id, the_name }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ColumnValue<RC>
where
    RC: Eq + Hash + ToString,
{
    String(RC),
    Float(f64),
    Date(NaiveDate),
}

// impl<T> ToString for ColumnValue<T>
// where
//     T: Eq + Hash + ToString,
// {
//     fn to_string(&self) -> String {
//     }
// }
impl<T> Display for ColumnValue<T>
where
    T: Eq + Hash + ToString,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            Self::String(v) => v.to_string(),
            Self::Float(v) => format!("{:.2}", v),
            Self::Date(v) => v.to_string(),
        };
        write!(f, "{}", result)
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
            (ColumnValue::String(s1), ColumnValue::String(s2)) => Some(s1.cmp(&s2)),
            (ColumnValue::Date(b1), ColumnValue::Date(b2)) => Some(b1.cmp(&b2)),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Row<I, RC>
where
    RC: Eq + Hash + ToString,
    I: IdMarker,
{
    pub id: I,
    pub columns: HashMap<RC, Column<RC>>,
}

impl ToSerial<Row<Arc<str>, Arc<str>>> for Row<Uuid, Arc<str>> {
    fn to_serial(self) -> Row<Arc<str>, Arc<str>> {
        let Row { id, columns } = self;
        let id = id.to_serial();
        Row { id, columns }
    }
}

pub trait RowsSort {
    fn sort_rows(&mut self, keys: Rc<[Rc<str>]>);
}

impl RowsSort for Vec<Row<Uuid, Rc<str>>> {
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

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct Sheet<I, RC>
where
    RC: Eq + Hash + ToString,
    I: IdMarker,
{
    pub id: I,
    pub sheet_name: RC,
    pub type_name: RC,
    pub insert_date: NaiveDate,
    pub rows: Vec<Row<I, RC>>,
}

impl ToSerial<Sheet<Arc<str>, Arc<str>>> for Sheet<Uuid, Arc<str>> {
    fn to_serial(self) -> Sheet<Arc<str>, Arc<str>> {
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
        Sheet {
            id,
            sheet_name,
            type_name,
            insert_date,
            rows,
        }
    }
}

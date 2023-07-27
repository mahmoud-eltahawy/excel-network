CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS sheets (
  id UUID PRIMARY KEY NOT NULL,
  sheet_name VARCHAR(80) NOT NULL,
  sheet_type VARCHAR(80) NOT NULL,
  insert_date DATE NOT NULL
);

CREATE INDEX IF NOT EXISTS sheets_names_idx ON sheets(sheet_name);
CREATE INDEX IF NOT EXISTS sheets_type_idx ON sheets(sheet_type);
CREATE INDEX IF NOT EXISTS sheets_insert_date_idx ON sheets(insert_date);

CREATE TABLE IF NOT EXISTS rows (
  id UUID PRIMARY KEY NOT NULL,
  sheet_id UUID NOT NULL,
  insert_date DATE DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (sheet_id) REFERENCES sheets (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS rows_sheet_id_idx ON rows(sheet_id);
CREATE INDEX IF NOT EXISTS rows_insert_date_idx ON rows(insert_date);

CREATE TABLE IF NOT EXISTS columns (
  id UUID PRIMARY KEY NOT NULL,
  row_id UUID NOT NULL,
  header_name VARCHAR(80) NOT NULL,
  value JSON NOT NULL,
  UNIQUE(row_id,header_name),
  FOREIGN KEY (row_id) REFERENCES rows (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS columns_row_id_idx ON columns(row_id);
CREATE INDEX IF NOT EXISTS columns_header_name_idx ON columns(header_name);

-- Kite Database Schema v4
-- Feature 005: metas de ahorro y apartados (movimiento tipo 'aporte').
-- Reconstruye movements para agregar 'aporte' al CHECK de tipo y la columna meta_id.

PRAGMA foreign_keys = OFF;

CREATE TABLE movements_new (
    id TEXT PRIMARY KEY,
    tipo TEXT NOT NULL CHECK(tipo IN ('ingreso','gasto','aporte')),
    monto_centavos INTEGER NOT NULL CHECK(monto_centavos > 0),
    concepto TEXT NOT NULL CHECK(trim(concepto) != ''),
    fecha TEXT NOT NULL,
    origen TEXT NOT NULL CHECK(origen IN ('formulario','mensaje')) DEFAULT 'formulario',
    meta_id TEXT,
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
);
INSERT INTO movements_new
    (id, tipo, monto_centavos, concepto, fecha, origen, meta_id, creado_en, actualizado_en)
SELECT id, tipo, monto_centavos, concepto, fecha, origen, NULL, creado_en, actualizado_en
FROM movements;
DROP TABLE movements;
ALTER TABLE movements_new RENAME TO movements;
CREATE INDEX idx_movements_fecha ON movements(fecha DESC, creado_en DESC);
CREATE INDEX idx_movements_tipo_fecha ON movements(tipo, fecha DESC);

CREATE TABLE IF NOT EXISTS metas (
    id TEXT PRIMARY KEY,
    nombre TEXT NOT NULL CHECK(trim(nombre) != ''),
    monto_objetivo_centavos INTEGER NOT NULL CHECK(monto_objetivo_centavos > 0),
    fecha_objetivo TEXT,
    estado TEXT NOT NULL CHECK(estado IN ('activa','completada','cancelada')),
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
);

PRAGMA user_version = 4;
PRAGMA foreign_keys = ON;

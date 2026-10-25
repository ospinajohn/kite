-- Kite Database Schema v1
-- Esquema inicial para la tabla de movimientos

CREATE TABLE IF NOT EXISTS movements (
    id TEXT PRIMARY KEY,
    tipo TEXT NOT NULL CHECK(tipo IN ('ingreso','gasto')),
    monto_centavos INTEGER NOT NULL CHECK(monto_centavos > 0),
    concepto TEXT NOT NULL CHECK(trim(concepto) != ''),
    fecha TEXT NOT NULL,
    origen TEXT NOT NULL CHECK(origen IN ('formulario','mensaje')) DEFAULT 'formulario',
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_movements_fecha ON movements(fecha DESC, creado_en DESC);
CREATE INDEX IF NOT EXISTS idx_movements_tipo_fecha ON movements(tipo, fecha DESC);

PRAGMA user_version = 1;
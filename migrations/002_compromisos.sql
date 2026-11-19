-- Kite Database Schema v2
-- Compromisos recurrentes y pagos (feature 002)

CREATE TABLE IF NOT EXISTS compromisos (
    id TEXT PRIMARY KEY,
    concepto TEXT NOT NULL CHECK(trim(concepto) != ''),
    monto_centavos INTEGER NOT NULL CHECK(monto_centavos > 0),
    frecuencia TEXT NOT NULL CHECK(frecuencia IN ('mensual','quincenal')),
    dia_mes INTEGER CHECK(dia_mes IS NULL OR (dia_mes BETWEEN 1 AND 31)),
    quincena TEXT CHECK(quincena IS NULL OR quincena IN ('primera','segunda')),
    duracion_tipo TEXT NOT NULL CHECK(duracion_tipo IN ('indefinido','fecha_fin','cuotas')),
    fecha_fin TEXT,
    cuotas_total INTEGER CHECK(cuotas_total IS NULL OR cuotas_total >= 1),
    cuotas_confirmadas INTEGER NOT NULL DEFAULT 0 CHECK(cuotas_confirmadas >= 0),
    estado TEXT NOT NULL DEFAULT 'activo' CHECK(estado IN ('activo','pausado')),
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL,
    CHECK(
        (duracion_tipo = 'indefinido' AND fecha_fin IS NULL AND cuotas_total IS NULL)
        OR (duracion_tipo = 'fecha_fin' AND fecha_fin IS NOT NULL AND cuotas_total IS NULL)
        OR (duracion_tipo = 'cuotas' AND fecha_fin IS NULL AND cuotas_total IS NOT NULL)
    ),
    CHECK(
        (frecuencia = 'mensual' AND dia_mes IS NOT NULL AND quincena IS NULL)
        OR (frecuencia = 'quincenal' AND dia_mes IS NULL AND quincena IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_compromisos_estado ON compromisos(estado, concepto);

CREATE TABLE IF NOT EXISTS compromisos_pagos (
    id TEXT PRIMARY KEY,
    compromiso_id TEXT NOT NULL REFERENCES compromisos(id) ON DELETE CASCADE,
    periodo_key TEXT NOT NULL,
    movimiento_id TEXT NOT NULL REFERENCES movements(id),
    monto_pagado INTEGER NOT NULL CHECK(monto_pagado > 0),
    fecha_pago TEXT NOT NULL,
    UNIQUE(compromiso_id, periodo_key)
);

CREATE INDEX IF NOT EXISTS idx_pagos_compromiso ON compromisos_pagos(compromiso_id, fecha_pago);

PRAGMA user_version = 2;

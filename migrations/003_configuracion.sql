-- Kite Database Schema v3
-- Configuración clave/valor (feature 003): por ahora `dia_corte` (1–28, default 15).

CREATE TABLE IF NOT EXISTS configuracion (
    clave TEXT PRIMARY KEY,
    valor TEXT NOT NULL
);

PRAGMA user_version = 3;
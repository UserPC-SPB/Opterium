#!/bin/bash
set -e

echo "============================================"
echo "  Opterium GeoFormer — Setup & Run"
echo "============================================"
echo

# 1. Проверяем Python
if ! command -v python3 &>/dev/null; then
    echo "ERROR: Python3 не найден!"
    echo "Установите: apt install python3 или brew install python"
    exit 1
fi

# 2. Устанавливаем cffi
echo "[1/3] Установка cffi..."
pip3 install cffi -q
echo "  OK"

# 3. Генерируем таблицы (если нет)
echo "[2/3] Генерация таблиц..."
if [ ! -f "src/tables.ptbl" ]; then
    python3 src/table_format.py
else
    echo "  Таблицы уже есть"
fi

# 4. Собираем Rust (если нет .so)
echo "[3/3] Сборка Rust..."
if [ ! -f "native/target/release/libgeofield.so" ] && [ ! -f "native/target/release/libgeofield.dylib" ]; then
    cd native
    cargo build --release
    cd ..
else
    echo "  Библиотека уже есть"
fi
echo

# 5. Запускаем demo
echo "============================================"
echo "  Запуск demo..."
echo "============================================"
echo
python3 demo.py

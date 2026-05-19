@echo off
echo ============================================
echo   Opterium GeoFormer — Setup ^& Run
echo ============================================
echo.

:: 1. Проверяем Python
python --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: Python не найден!
    echo Установите Python 3.10+ с https://python.org
    pause
    exit /b 1
)

:: 2. Устанавливаем cffi
echo [1/3] Установка cffi...
pip install cffi -q
if %errorlevel% neq 0 (
    echo ERROR: не удалось установить cffi
    pause
    exit /b 1
)
echo   OK

:: 3. Генерируем таблицы (если нет)
echo [2/3] Генерация таблиц...
if not exist "src\tables.ptbl" (
    python src\table_format.py
    if %errorlevel% neq 0 (
        echo ERROR: не удалось сгенерировать таблицы
        pause
        exit /b 1
    )
) else (
    echo   Таблицы уже есть
)

:: 4. Собираем Rust (если нет DLL)
echo [3/3] Сборка Rust...
if not exist "native\target\release\geofield.dll" (
    cd native
    cargo build --release
    if %errorlevel% neq 0 (
        echo ERROR: не удалось собрать Rust
        echo Установите Rust с https://rustup.rs
        cd ..
        pause
        exit /b 1
    )
    cd ..
) else (
    echo   DLL уже есть
)
echo.

:: 5. Запускаем demo
echo ============================================
echo   Запуск demo...
echo ============================================
echo.
python demo.py
echo.
pause

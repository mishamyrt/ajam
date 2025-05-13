use ajam_events::active_space::{ActivityMonitor, ActivityEvent};
use std::time::Duration;

fn main() {
    let (monitor, rx) = ActivityMonitor::new();

    // Запускаем монитор в фоновом потоке, не блокируя основной поток
    let monitor_thread = monitor.start_listening_background();

    println!("Ожидание событий активности в фоновом режиме...");
    println!("Нажмите Ctrl+C для выхода");

    // В основном потоке слушаем события и продолжаем выполнять другую работу
    loop {
        // Обрабатываем события без блокировки с таймаутом
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ActivityEvent::AppChange(bundle_id)) => {
                println!("Активное приложение изменилось: {}", bundle_id);
            }
            Ok(ActivityEvent::AudioInputChange(device_name)) => {
                println!("Входное аудиоустройство изменилось: {}", device_name);
            }
            Ok(ActivityEvent::AudioOutputChange(device_name)) => {
                println!("Выходное аудиоустройство изменилось: {}", device_name);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Продолжаем цикл при таймауте
                // Здесь можно выполнять другую работу
                std::thread::sleep(Duration::from_millis(10)); // Чтобы не загружать процессор
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("Канал событий закрыт");
                break;
            }
        }
        
        // Пример другой работы, которую можно выполнять в основном потоке
        // ...
    }
} 
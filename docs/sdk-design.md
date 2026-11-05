# Единый дизайн SDK

**Статус:** Proposal. **Ни один показанный API пока не реализован.**  
**Языки:** C++, Python, Rust, Go, Java, TypeScript/JavaScript.

## Договор о семантике

Для всех SDK должны быть одинаково определены:

- выбор транспортного DDS DomainId и конфигурации discovery;
- publish/subscribe для генерируемого типизированного Topic;
- явно выбираемый preset QoS с override;
- сериализация и проверка совместимости типов по IDL;
- владение ресурсами и завершение подписки;
- backpressure, cancellation и обработка ошибок;
- семантика `publish`: возвращение из вызова **не равнозначно** durable persistence или успешному выполнению обработчика.

Различия синтаксиса допустимы, различия wire-типов — нет.

## Python

Целевой интерфейс — async context manager, `async for` и явная модель QoS. Реализация не должна блокировать asyncio event loop внутри callback. Поддержка sync API может появиться отдельно.

```python
# PROPOSED ONLY: not an importable package yet.
from d2s import DataBus
from sensors_generated import Temperature

async with DataBus(domain=10) as bus:
    async with bus.subscribe("sensors.temperature", Temperature) as stream:
        async for msg in stream:
            print(msg.value)
```

Примечание: `Temperature` обязан иметь сгенерированное DDS-сопоставление; произвольный dataclass не становится wire-типом автоматически.

## Rust

Предполагаемый API — типобезопасные `Publisher<T>` / `Subscriber<T>`, `Stream<Item=Result<T, Error>>`, Tokio integration, явное `close()`/Drop. FFI-граница не передаёт Rust lifetime в C++; инварианты Send/Sync и безопасное отключение callbacks проверяются. Отмена задач не должна приводить к use-after-free.

## Go

`context.Context` для отмены и deadline, `Publish(ctx, value)`, `Subscribe(ctx, handler)` или типизированный iterator/channel. CGo требует осторожного обращения с указателями и callback threads; buffers ограничены, чтобы медленный подписчик не вызывал утечку памяти.

## C++

Нативный RAII API с типами генератора IDL, без обязательного дополнительного копирования payload и с явным ownership. Пользователь должен иметь escape hatch к Fast DDS API для специализированных QoS и инструментирования.

## Java

JNI/native adapter с безопасной очисткой ресурсов; типизированные publisher/subscriber, `Flow.Publisher<T>` и `CompletableFuture` в соответствующих API. Следить за нагрузкой на GC, класс-лоадеры и завершением нативных потоков.

## TypeScript / JavaScript

Для браузера и стандартного web-клиента — `WebSocket/HTTP Gateway` с отдельной аутентификацией и wire contract. Не обещать прямое DDS/RTPS-соединение из браузера. Типы генерируются из канонической схемы, но transport envelope отдельно версионируется.

```typescript
// PROPOSED ONLY: works through future gateway.
const client = await D2S.connect({
  gateway: "wss://gateway.example",
  token: "<issued-token>"
});
const sub = client.subscribe<Temperature>("sensors.temperature");
for await (const message of sub) {
  console.log(message.value);
}
await sub.close();
```

## Native ABI

Предполагается отдельная ABI-граница для языков с FFI. Не считать ABI Fast DDS стабильным API для сторонних SDK. Нужны opaque handles, явные create/destroy, versioned error codes, thread safety specification, callback ownership, type descriptors и ABI compatibility tests.

## Ошибки и lifecycle

Общий каталог ошибок: `DiscoveryUnavailable`, `QoSIncompatible`, `SchemaMismatch`, `AuthorizationDenied`, `TransportError`, `ResourceLimit`, `Closed`, `Timeout`. Это целевые категории, а не утверждённые исключения.

Требования: deterministic cleanup; no silent drop beyond selected QoS; bounded buffers; отдельные timeout и cancellation; непустая диагностическая информация без раскрытия секретов.

## Первый комплект

Начать с C++ и Python. Только после успешного межъязыкового теста добавлять Rust и Go. Java и TS gateway позже; избежать ситуации «шесть фасадов, но ни одного работающего».

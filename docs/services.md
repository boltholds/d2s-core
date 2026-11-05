# Дополнительные службы платформы

**Статус:** Deferred / Proposal. Наличие документа не означает наличия службы в репозитории.

## Routing / Domain Bridge

Используется для обмена между независимыми DDS-доменами и сегментами сети; допускается интеграция с [eProsima DDS Router](https://github.com/eProsima/DDS-Router). В D2S требуются независимые политики доступа, метрики и контроль транзита. Конфигурация Router не считается автоматически эквивалентной federation graph policy.

## Web Gateway

HTTP/WebSocket адаптер для браузерных клиентов и языков без нативного DDS. Gateway — отдельная доверенная граница. Нужны authn/authz, schema validation, ограничение нагрузки, versioned protocol, max message size и явные semantics backpressure/reconnect.

## Persistence / Replay

Возможные основы: существующие persistence/recording решения DDS, [eProsima DDS Record & Replay](https://github.com/eProsima/DDS-Record-Replay). Не путать transient-local state с распределённым append-only журналом. Точное подтверждение durable write, retention, replay-offset и повторная обработка — отдельные будущие контракты.

## Observability

Минимум:
- локальные peers, Topics, endpoint matching, QoS conflicts;
- publish/receive throughput, p50/p95/p99 latency, queue depth;
- состояния соседства и маршрутов федерации;
- permitted/denied cross-domain operations (без утечки payload);
- возможность экспортировать метрики Prometheus и trace контекст OpenTelemetry там, где это возможно.

## CLI / Control Plane

Возможные команды `d2s doctor`, `d2s peers`, `d2s topics`, `d2s qos validate`, `d2s federation routes` — **только имена для проектирования**. Никакого global admin server для local mode не требуется.

## Правило развёртывания

Библиотека local Pub/Sub обязана работать без установки перечисленных служб. Каждая служба должна иметь отдельные runtime assumptions, security boundary и failure modes.

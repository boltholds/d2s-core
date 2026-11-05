# План реализации D2S Platform

**Статус:** Proposed roadmap; даты и обещания релиза отсутствуют.

## Phase 0 — архитектурный спайк

- Подтвердить версию Fast DDS, лицензии и требования toolchain.
- Вручную собрать минимальные два C++ DDS process с общим IDL и QoS.
- Измерить baseline latency и memory.
- Решить: thin wrapper над Fast DDS или стабильный C ABI.
- Зафиксировать RFC и архитектурные решения перед кодом.

**Готово, если:** можно воспроизвести интеграционный тест без D2S abstractions.

## Phase 1 — Local Core (MVP)

- Native runtime + корректная обработка ресурсов.
- C++ SDK и Python SDK.
- Общие IDL типы и генерация для этих двух языков.
- Pub/Sub, базовые QoS presets, диагностика matching.
- Демонстрация двух устройств в одной LAN без брокера.

**Готово, если:** C++ публикует, Python читает и наоборот, после рестарта peer обмен восстанавливается в пределах выбранного QoS.

## Phase 2 — Multi-language SDK

- Rust (FFI и Tokio), Go (CGo и context).
- Межъязыковые round-trip тесты, матрица типов.
- Версионирование внутреннего ABI и контракт ошибок.
- Репродуцируемые сборки и пакеты.

## Phase 3 — Connectivity & DX

- Initial peers, подсети/VPN, документация сетевого развёртывания.
- WebSocket gateway + TypeScript SDK.
- Java JNI SDK.
- CLI diagnostics, telemetry, basic examples.
- Измерение overhead относительно прямого Fast DDS.

## Phase 4 — Federation research and prototype

- RFC логических идентичностей доменов и trust model.
- Граф с ограниченной видимостью соседей.
- Scoped route discovery и default-deny authorizations.
- Предотвращение циклов, ограничение трафика и recovery.
- Сравнение с существующим DDS Router и более простой federated architecture.

**Не продолжать к production**, если недоказана безопасность политик или отсутствует реальное преимущество.

## Phase 5 — Optional platform services

- Persistence, recording/replay, extended observability.
- Нагрузочные и отказоустойчивые испытания.
- Документация эксплуатации и threat model update.

## Приоритеты

Безопасность и понятные гарантии доставки — часть самого MVP. Глобальная P2P-сеть, распределённое хранение и автоматический NAT traversal — отдельные крупные проекты и не должны блокировать начало локальной реализации.

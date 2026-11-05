# QoS и семантика доставки

**Статус:** Proposal; использует базовые понятия DDS.

## Профили для SDK

| D2S preset | Исходная идея | Ограничения |
|---|---|---|
| `realtime` | BestEffort + KeepLast | Потери допустимы, подходит для часто обновляемого состояния |
| `reliable` | Reliable + ограниченная History/ResourceLimits | Не означает сохранения на диск или успешной обработки |
| `state` | TransientLocal + KeepLast | Поздний читатель получает доступное состояние при живом подходящем writer |
| `persistent` | Плагин долговременного хранения поверх DDS | Deferred; требует явного durability contract |
| `custom` | Явные DDS QoS | Для опытных пользователей |

Названия и точные значения профилей до реализации — проектные.

## Какие подтверждения существуют

Не смешивать:

1. **Accepted locally** — SDK принял publish вызов/буфер.
2. **Written to DDS** — данные переданы DataWriter согласно API.
3. **Acknowledged by matched readers** — транспортные DDS подтверждения в пределах QoS и ожидания.
4. **Persisted** — независимый сервис зафиксировал данные в согласованном долговременном хранилище.
5. **Processed** — приложение подтвердило выполнение бизнес-операции.

Пункты 4 и 5 не выводятся из одного `RELIABILITY=RELIABLE`. «Exactly once» для пользовательской бизнес-логики без дополнительного протокола не обещается.

## Late joiners и HISTORY

DDS `TRANSIENT_LOCAL` и `HISTORY` позволяют сохранить доступное состояние/некоторую историю в пределах соответствующих ресурсов и времени жизни writer. Они не равнозначны Kafka-style журналу с произвольным replay offset.

## Failure modes

Документировать для каждого SDK:
- поведение при publisher loss, subscriber loss, slow reader, network partition;
- потери при переполнении очередей, deadline и ресурсных ограничениях;
- retry/backpressure/cancellation;
- условия, при которых publish возвращает управление;
- причины несовместимости QoS writer/reader.

## Федерация

Сохранение профиля QoS от исходного writer до получателя через несколько мостов нельзя обещать автоматически: каждый переход имеет свои границы подтверждения, буферы и отказные состояния. Междоменные гарантии требуют отдельного контракта и интеграционных тестов.

## Инструментирование

Минимальные измерения: delivered/sent ratio, reader queue age, publish latency, matching time, deadline misses, QoS incompatibility и memory pressure. Benchmark только с указанными transport, payload size, QoS, ОС и устройствами.

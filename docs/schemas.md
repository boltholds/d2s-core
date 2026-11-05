# Схемы данных и межъязыковая совместимость

**Статус:** Proposal.

## Источник истины

Основной договорённый кандидат — **DDS IDL**, а не независимые Python dataclass / Go struct / Rust struct. Генератор создаёт удобные модели и адаптеры для всех SDK; по проводу передаются совместимые DDS-типы. Перед фиксацией генератора нужно испытать версии IDL, XTypes, ключи экземпляров, optional/union/sequence и поддерживаемые реализации Fast DDS.

```idl
// Illustration; validation with the chosen IDL compiler required
module sensors {
  struct Temperature {
    string device_id;
    double value;
  };
};
```

Предлагаемый цикл: `schema.idl` → validation → canonical schema descriptor → language generators → common interop tests → package artifacts.

## Что входит в contract tests

- Python publisher → C++/Rust/Go readers и обратные направления.
- Проверка имени Topic, type identifier, keys и matching QoS.
- Primitive types, strings, sequences, arrays, nested structs и edge cases.
- Преобразования знаковых/беззнаковых чисел, float, unicode/encoding.
- Совместимость evolving schemas: добавление/удаление/переименование полей только по явно документированным правилам DDS-XTypes.
- Negatives: несовместимая схема, неверный type identifier, неопределённая политика evolution.
- Матрица версий SDK и native Fast DDS.

## Жизненный цикл схем

Пока не утверждены schema registry, глобальные имена и wire-версионирование. Упоминание типа `Temperature` в примере SDK само по себе не обеспечивает регистрацию DDS-типа. Нужен генератор/адаптер и явное сопоставление со схемой.

Управление версиями:
1. Изменение IDL сопровождается совместимостью на уровне wire.
2. CI проверяет сгенерированные артефакты.
3. Версия schema-package не обязана совпадать с версией SDK.
4. В runtime ошибки несовместимых схем не маскируются как «нет подписчиков».

## Неприемлемо

Создание отдельного JSON-only транспорта для одного SDK без явного gateway-контракта; принудительная сериализация в Python pickle; обещание полной совместимости любой версии DDS-XTypes до тестирования.

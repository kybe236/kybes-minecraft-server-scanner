package de.kybe.settings;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

public class NumberSetting<T extends Number> extends Setting<T> {
  private T value;

  public NumberSetting(String name, T defaultValue) {
    super(name);
    this.value = defaultValue;
  }

  @Override
  public JsonObject toJson() {
    JsonObject obj = new JsonObject();
    obj.addProperty("type", "number");
    obj.addProperty("name", getName());
    obj.addProperty("value", value.toString());

    JsonArray sub = new JsonArray();
    for (Setting<?> setting : getSubSettings()) {
      sub.add(setting.toJson());
    }
    obj.add("subSettings", sub);
    return obj;
  }

  @Override
  @SuppressWarnings("unchecked")
  public void fromJson(JsonObject json) {
    try {
      if (value instanceof Integer) {
        setValue((T) (Integer) json.get("value").getAsInt());
      } else if (value instanceof Double) {
        setValue((T) (Double) json.get("value").getAsDouble());
      } else if (value instanceof Float) {
        setValue((T) (Float) json.get("value").getAsFloat());
      } else if (value instanceof Long) {
        setValue((T) (Long) json.get("value").getAsLong());
      }
    } catch (Exception ignored) {
    }

    loadSubSettings(json);
  }

  public T parseValue(String str) throws NumberFormatException {
    switch (value) {
      case Integer ignored -> setValue((T) (Integer) Integer.parseInt(str));
      case Double ignored -> setValue((T) (Double) Double.parseDouble(str));
      case Float ignored -> setValue((T) (Float) Float.parseFloat(str));
      case Long ignored -> setValue((T) (Long) Long.parseLong(str));
      default -> throw new NumberFormatException("Unsupported number type: " + value.getClass());
    }
    return getValue();
  }


  public T getValue() {
    return value;
  }

  public void setValue(T value) {
    T old = this.value;
    this.value = value;
    notifyChange(old, value);
  }

}

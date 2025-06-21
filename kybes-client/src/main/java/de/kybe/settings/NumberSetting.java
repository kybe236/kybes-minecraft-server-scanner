package de.kybe.settings;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
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
  public void fromJson(JsonObject json) {
    try {
      if (value instanceof Integer) {
        setValue((T)(Integer) json.get("value").getAsInt());
      } else if (value instanceof Double) {
        setValue((T)(Double) json.get("value").getAsDouble());
      }
    } catch (Exception ignored) {}

    loadSubSettings(json);
  }

  public T getValue() { return value; }
  public void setValue(T value) { this.value = value; }
}

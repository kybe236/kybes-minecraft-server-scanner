package de.kybe.settings;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

public class StringSetting extends Setting<String>{
  private String value;

  public StringSetting(String name, String defaultValue) {
    super(name);
    this.value = defaultValue;
  }

  @Override
  public JsonObject toJson() {
    JsonObject obj = new JsonObject();
    obj.addProperty("type", "string");
    obj.addProperty("name", getName());
    obj.addProperty("value", value);

    JsonArray sub = new JsonArray();
    for (Setting<?> setting : getSubSettings()) {
      sub.add(setting.toJson());
    }
    obj.add("subSettings", sub);
    return obj;
  }

  @Override
  public void fromJson(JsonObject json) {
    setValue(json.get("value").getAsString());

    loadSubSettings(json);
  }

  public String getValue() { return value; }
  public void setValue(String value) { this.value = value; }
}

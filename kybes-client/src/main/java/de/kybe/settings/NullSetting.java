package de.kybe.settings;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

public class NullSetting extends Setting<Void> {
  public NullSetting(String name) {
    super(name);
  }

  @Override
  public JsonObject toJson() {
    JsonObject obj = new JsonObject();
    obj.addProperty("type", "null");
    obj.addProperty("name", getName());

    JsonArray sub = new JsonArray();
    for (Setting<?> subSetting : getSubSettings()) {
      sub.add(subSetting.toJson());
    }
    obj.add("subSettings", sub);

    return obj;
  }

  @Override
  public void fromJson(JsonObject json) {
    loadSubSettings(json);
  }

  public Void getValue() { return null; }
  public void setValue(Void value) {}
}

package de.kybe.settings;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

public class BindSetting extends Setting<Integer> {
  private int keyCode;

  public BindSetting(String name, int defaultKeyCode) {
    super(name);
    this.keyCode = defaultKeyCode;
  }

  @Override
  public Integer getValue() {
    return keyCode;
  }

  @Override
  public void setValue(Integer value) {
    int old = this.keyCode;
    this.keyCode = value;
    notifyChange(old, value);
  }

  @Override
  public JsonObject toJson() {
    JsonObject obj = new JsonObject();
    obj.addProperty("name", getName());
    obj.addProperty("keyCode", keyCode);

    JsonArray sub = new JsonArray();
    for (Setting<?> subSetting : getSubSettings()) {
      sub.add(subSetting.toJson());
    }
    obj.add("subSettings", sub);

    return obj;
  }

  @Override
  public void fromJson(JsonObject json) {
    if (json.has("keyCode")) {
      setValue(json.get("keyCode").getAsInt());
    }
    loadSubSettings(json);
  }
}

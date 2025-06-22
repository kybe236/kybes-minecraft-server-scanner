package de.kybe.settings;

import com.google.gson.JsonObject;

public class BooleanSetting extends Setting<Boolean> {
  private boolean value;

  public BooleanSetting(String name, boolean defaultValue) {
    super(name);
    this.value = defaultValue;
  }

  @Override
  public Boolean getValue() {
    return value;
  }

  @Override
  public void setValue(Boolean value) {
    Boolean old = this.value;
    this.value = value;
    notifyChange(old, value);
  }

  @Override
  public JsonObject toJson() {
    JsonObject json = new JsonObject();
    json.addProperty("name", getName());
    json.addProperty("value", value);

    // Serialize subSettings if any
    if (!getSubSettings().isEmpty()) {
      var subSettingsArray = new com.google.gson.JsonArray();
      for (Setting<?> sub : getSubSettings()) {
        subSettingsArray.add(sub.toJson());
      }
      json.add("subSettings", subSettingsArray);
    }
    return json;
  }

  @Override
  public void fromJson(JsonObject json) {
    if (json.has("value")) {
      this.value = json.get("value").getAsBoolean();
    }
    // Load subSettings if present
    loadSubSettings(json);
  }
}
package de.kybe.settings;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.ArrayList;
import java.util.List;

public abstract class Setting<T> {
  private final String name;
  private final List<Setting<?>> subSettings = new ArrayList<>();

  public Setting(String name) {
    this.name = name;
  }

  public String getName() { return name; }
  public List<Setting<?>> getSubSettings() { return subSettings; }

  public void addSubSetting(Setting<?> setting) {
    subSettings.add(setting);
  }

  protected void loadSubSettings(JsonObject json) {
    if (!json.has("subSettings")) return;

    for (JsonElement el : json.getAsJsonArray("subSettings")) {
      JsonObject subJson = el.getAsJsonObject();
      String name = subJson.get("name").getAsString();
      for (Setting<?> sub : getSubSettings()) {
        if (sub.getName().equals(name)) {
          sub.fromJson(subJson);
          break;
        }
      }
    }
  }


  public abstract T getValue();
  public abstract void setValue(T value);

  public abstract JsonObject toJson();
  public abstract void fromJson(JsonObject json);
}

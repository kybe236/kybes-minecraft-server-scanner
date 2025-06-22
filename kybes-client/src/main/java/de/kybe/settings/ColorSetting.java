package de.kybe.settings;

import com.google.gson.JsonObject;

public class ColorSetting extends Setting<Integer> {
  private int color;

  public ColorSetting(String name, int defaultColor) {
    super(name);
    this.color = defaultColor;
  }

  @Override
  public Integer getValue() {
    return color;
  }

  @Override
  public void setValue(Integer value) {
    int old = this.color;
    this.color = value;
    notifyChange(old, value);
  }

  // Your existing hex parser that adds alpha if missing
  public void setHex(String hex) {
    if (hex == null) return;
    try {
      hex = hex.trim();
      if (hex.startsWith("#")) hex = hex.substring(1);
      if (hex.length() == 6) hex += "FF"; // add default alpha
      if (hex.length() != 8) return;
      int parsed = (int) Long.parseLong(hex, 16);
      setValue(parsed);
    } catch (NumberFormatException ignored) {}
  }

  // New method to accept hex string directly
  public void setValue(String hex) {
    setHex(hex);
  }

  public String getHex() {
    return String.format("#%08X", color);
  }

  @SuppressWarnings("unused")
  public int getRed() {
    return (color >> 16) & 0xFF;
  }

  @SuppressWarnings("unused")
  public int getGreen() {
    return (color >> 8) & 0xFF;
  }

  @SuppressWarnings("unused")
  public int getBlue() {
    return color & 0xFF;
  }

  @SuppressWarnings("unused")
  public int getAlpha() {
    return (color >> 24) & 0xFF;
  }

  @Override
  public JsonObject toJson() {
    JsonObject json = new JsonObject();
    json.addProperty("name", getName());
    json.addProperty("value", getHex());

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
      setHex(json.get("value").getAsString());
    }
    loadSubSettings(json);
  }
}
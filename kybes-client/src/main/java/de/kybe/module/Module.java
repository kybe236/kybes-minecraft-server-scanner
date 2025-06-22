package de.kybe.module;

import de.kybe.settings.Setting;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public class Module {
  protected final String name;
  protected final List<Setting<?>> settings = new ArrayList<>();

  public Module(String name) {
    this.name = name;
  }

  public String getName() {
    return name;
  }

  public List<Setting<?>> getSettings() {
    return settings;
  }

  public void addSetting(Setting<?>... newSettings) {
    settings.addAll(Arrays.asList(newSettings));
  }

  /**
   * Called after the module is registered and fully loaded.
   */
  public void onLoad() {

  }
}

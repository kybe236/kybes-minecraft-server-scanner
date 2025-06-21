package de.kybe.module;

import de.kybe.settings.Setting;

import java.util.ArrayList;
import java.util.List;

public class Module {
  protected final String name;
  protected final List<Setting<?>> settings = new ArrayList<>();

  public Module(String name) {
    this.name = name;
    ModuleManager.register(this);
  }

  public String getName() { return name; }
  public List<Setting<?>> getSettings() { return settings; }
  public void addSetting(Setting<?> setting) { settings.add(setting); }
}

package de.kybe.module.modules;

import de.kybe.config.Config;
import de.kybe.event.KybeEvents;
import de.kybe.event.events.EventTick;
import de.kybe.module.Module;
import de.kybe.module.ModuleManager;
import de.kybe.settings.BooleanSetting;
import de.kybe.settings.NumberSetting;

public class ConfigModule extends Module {
  private final BooleanSetting autoSave = (BooleanSetting) new BooleanSetting("Auto Save", true)
    .onChange((oldValue, newValue) -> Config.setAutoSave(newValue));
  @SuppressWarnings("unchecked")
  private final NumberSetting<Integer> autoSaveIntervalTicks = (NumberSetting<Integer>) new NumberSetting<>("Auto Save Interval Ticks", 20 * 15)
    .onChange((oldValue, newValue) -> Config.setAutoSaveInterval(newValue));
  public ConfigModule() {
    super("Config");
    this.addSetting(load, save, autoSave, autoSaveIntervalTicks);
  }  public final BooleanSetting load = (BooleanSetting) new BooleanSetting("Load", false)
    .onChange((oldValue, newValue) -> {
      if (newValue) {
        ((ConfigModule) ModuleManager.getByName("Config")).load.setValue(false);
        Config.load();
      }
    });

  @Override
  public void onLoad() {
    Config.setAutoSave(autoSave.getValue());
    Config.setAutoSaveInterval(autoSaveIntervalTicks.getValue());
  }

  @KybeEvents
  @SuppressWarnings("unused")
  public void onTick(EventTick ignored) {
    Config.tick();
  }

  private final BooleanSetting save = (BooleanSetting) new BooleanSetting("Save", false)
    .onChange((oldValue, newValue) -> {
      if (newValue) {
        ((ConfigModule) ModuleManager.getByName("Config")).save.setValue(false);
        Config.save();
      }
    });
}

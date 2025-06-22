package de.kybe;

import de.kybe.config.Config;
import de.kybe.module.Module;
import de.kybe.module.ModuleManager;
import de.kybe.module.modules.ConfigModule;
import de.kybe.module.modules.GUIModule;
import de.kybe.module.modules.Scanner;
import de.kybe.module.modules.SharedConstants;
import net.fabricmc.api.ModInitializer;

public class KybesClient implements ModInitializer {
  @Override
  public void onInitialize() {

    ModuleManager.register(new GUIModule());
    ModuleManager.register(new SharedConstants());
    ModuleManager.register(new ConfigModule());
    ModuleManager.register(new Scanner());
    Config.load();
    ModuleManager.getAll().forEach(Module::onLoad);
  }
}
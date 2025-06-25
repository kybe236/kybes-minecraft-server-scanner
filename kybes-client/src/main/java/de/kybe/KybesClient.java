package de.kybe;

import de.kybe.command.CommandManager;
import de.kybe.command.commands.ToggleCommand;
import de.kybe.command.commands.VClipCommand;
import de.kybe.config.Config;
import de.kybe.module.Module;
import de.kybe.module.ModuleManager;
import de.kybe.module.modules.*;
import net.fabricmc.api.ModInitializer;

public class KybesClient implements ModInitializer {
  @Override
  public void onInitialize() {
    // MODULES
    ModuleManager.register(new GUIModule());
    ModuleManager.register(new SharedConstants());
    ModuleManager.register(new ConfigModule());
    ModuleManager.register(new ScannerModule());
    ModuleManager.register(new CrackedModule());
    ModuleManager.register(new SpeedModule());
    ModuleManager.register(new FlightModule());
    ModuleManager.register(new NoFallModule());
    ModuleManager.register(new AutoCommandModule());

    // COMMANDS
    CommandManager.register(new ToggleCommand());
    CommandManager.register(new VClipCommand());

    Config.load();
    ModuleManager.getAll().forEach(Module::onLoad);
  }
}
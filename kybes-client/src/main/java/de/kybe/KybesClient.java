package de.kybe;

import de.kybe.command.CommandManager;
import de.kybe.command.commands.ToggleCommand;
import de.kybe.config.Config;
import de.kybe.module.Module;
import de.kybe.module.ModuleManager;
import de.kybe.module.modules.*;
import net.fabricmc.api.ModInitializer;
import net.minecraft.network.protocol.game.ClientboundPlayerChatPacket;

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

    // COMMANDS
    CommandManager.register(new ToggleCommand());

    Config.load();
    ModuleManager.getAll().forEach(Module::onLoad);
  }
}
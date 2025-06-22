package de.kybe;

import de.kybe.config.Config;
import de.kybe.module.Module;
import de.kybe.module.ModuleManager;
import de.kybe.module.modules.*;
import net.fabricmc.api.ModInitializer;
import net.minecraft.network.protocol.game.ClientboundAddEntityPacket;

import static de.kybe.Constants.mc;

public class KybesClient implements ModInitializer {
  @Override
  public void onInitialize() {
    ModuleManager.register(new GUIModule());
    ModuleManager.register(new SharedConstants());
    ModuleManager.register(new ConfigModule());
    ModuleManager.register(new Scanner());
    ModuleManager.register(new CrackedModule());
    ModuleManager.register(new SpeedModule());
    ModuleManager.register(new FlightModule());
    ModuleManager.register(new NoFallModule());
    Config.load();
    ModuleManager.getAll().forEach(Module::onLoad);
  }
}
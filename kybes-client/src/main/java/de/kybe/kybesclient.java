package de.kybe;

import de.kybe.config.Config;
import net.fabricmc.api.ModInitializer;

public class kybesclient implements ModInitializer {

  @Override
  public void onInitialize() {
    Config.load();
  }
}

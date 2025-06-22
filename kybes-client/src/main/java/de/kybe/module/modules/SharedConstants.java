package de.kybe.module.modules;

import de.kybe.module.Module;
import de.kybe.settings.BooleanSetting;

public class SharedConstants extends Module {
  public BooleanSetting IS_RUNNING_IN_IDE = (BooleanSetting) new BooleanSetting("Running in IDE", false).onChange((oldValue, newValue) -> net.minecraft.SharedConstants.IS_RUNNING_IN_IDE = newValue);

  public SharedConstants() {
    super("SharedConstants");

    this.addSetting(IS_RUNNING_IN_IDE);
  }

  @Override
  public void onLoad() {
    net.minecraft.SharedConstants.IS_RUNNING_IN_IDE = IS_RUNNING_IN_IDE.getValue();
  }
}

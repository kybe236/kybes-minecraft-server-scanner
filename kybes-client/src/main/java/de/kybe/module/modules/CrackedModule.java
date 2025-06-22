package de.kybe.module.modules;

import de.kybe.module.ToggleableModule;
import de.kybe.settings.StringSetting;

public class CrackedModule extends ToggleableModule {
  public StringSetting username = new StringSetting("Username", "2kybe3");

  public CrackedModule() {
    super("Cracked");

    this.addSetting(username);
  }

  @Override
  protected void onToggled(boolean toggled) {
    if (toggled) {
      de.kybe.utils.CrackedUtils.login(username.getValue());
      setToggled(false);
    }
  }
}

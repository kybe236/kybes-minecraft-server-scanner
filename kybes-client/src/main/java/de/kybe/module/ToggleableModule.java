package de.kybe.module;

import de.kybe.settings.BindSetting;

public class ToggleableModule extends Module {
  private final BindSetting toggleBind;
  private boolean toggled;

  public ToggleableModule(String name) {
    super(name);

    toggleBind = new BindSetting("Bind", -1);

    addSetting(toggleBind);
  }

  public BindSetting getToggleBind() {
    return toggleBind;
  }

  public void checkToggle(int keyPressed) {
    if (keyPressed == toggleBind.getValue()) {
      setToggled(!isToggled());
    }
  }

  public boolean isToggled() {
    return toggled;
  }

  public void setToggled(boolean toggled) {
    this.toggled = toggled;
    onToggled(toggled);
  }

  public void toggle() {
    setToggled(!isToggled());
    onToggled(isToggled());
  }

  protected void onToggled(boolean toggled) {

  }
}
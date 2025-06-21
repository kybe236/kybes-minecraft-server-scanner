package de.kybe.module;

public class ToggleableModule extends Module {
  private boolean toggled;

  public ToggleableModule(String name) {
    super(name);
  }

  public void setToggled(boolean toggled) {
    this.toggled = toggled;
  }

  public boolean isToggled() {
    return toggled;
  }
}

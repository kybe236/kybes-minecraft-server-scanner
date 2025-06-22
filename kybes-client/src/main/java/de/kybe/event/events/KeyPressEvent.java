package de.kybe.event.events;

public class KeyPressEvent extends CancelableEvent {
  private final int key;
  private final int scancode;
  private final int modifiers;

  public KeyPressEvent(int key, int scancode, int modifiers) {
    this.key = key;
    this.scancode = scancode;
    this.modifiers = modifiers;
  }

  @SuppressWarnings("unused")
  public int getKey() {
    return key;
  }

  @SuppressWarnings("unused")
  public int getScancode() {
    return scancode;
  }

  @SuppressWarnings("unused")
  public int getModifiers() {
    return modifiers;
  }
}

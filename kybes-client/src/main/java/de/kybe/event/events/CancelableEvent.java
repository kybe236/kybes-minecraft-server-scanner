package de.kybe.event.events;

public class CancelableEvent extends Event {
  private boolean cancelled = false;
  public boolean isCancelled() { return cancelled; }
  public void setCancelled(boolean cancelled) { this.cancelled = cancelled; }
}
package de.kybe.module.modules;

import de.kybe.event.KybeEvents;
import de.kybe.event.events.EventTick;
import de.kybe.module.ToggleableModule;
import de.kybe.settings.BooleanSetting;
import de.kybe.settings.NumberSetting;
import net.minecraft.world.phys.Vec3;

import static de.kybe.Constants.mc;

public class FlightModule extends ToggleableModule {
  public final NumberSetting<Double> verticalSpeed = new NumberSetting<>("Vertical Speed", 2.0);
  public final NumberSetting<Double> horizontalSpeed = new NumberSetting<>("Horizontal Speed", 2.0);
  public final BooleanSetting antiKick = new BooleanSetting("AntiKick", true);

  private int tickCounter = 0;

  public FlightModule() {
    super("Flight");

    this.addSetting(verticalSpeed, horizontalSpeed, antiKick);
  }

  @KybeEvents
  @SuppressWarnings("unused")
  public void onTick(EventTick event) {
    if (!this.isToggled()) return;
    if (mc.player == null) return;

    tickCounter++;
    if (tickCounter > 20) tickCounter = 0;

    double vSpeed = verticalSpeed.getValue();
    double hSpeed = horizontalSpeed.getValue();

    boolean forward = mc.options.keyUp.isDown();
    boolean back = mc.options.keyDown.isDown();
    boolean left = mc.options.keyLeft.isDown();
    boolean right = mc.options.keyRight.isDown();
    boolean jump = mc.options.keyJump.isDown();
    boolean sneak = mc.options.keyShift.isDown();

    Vec3 lookVec = mc.player.getLookAngle();
    Vec3 motion = new Vec3(0, 0, 0);

    Vec3 forwardVec = new Vec3(lookVec.x, 0, lookVec.z).normalize().scale(hSpeed);
    Vec3 rightVec = new Vec3(-lookVec.z, 0, lookVec.x).normalize().scale(hSpeed);

    if (forward) motion = motion.add(forwardVec);
    if (back) motion = motion.subtract(forwardVec);
    if (right) motion = motion.add(rightVec);
    if (left) motion = motion.subtract(rightVec);

    if (jump) motion = motion.add(0, vSpeed, 0);
    if (sneak) motion = motion.subtract(0, vSpeed, 0);

    if (antiKick.getValue() && tickCounter % 5 == 0) {
      motion = motion.subtract(0, 0.2, 0);
    } else if (antiKick.getValue() && tickCounter % 10 == 0) {
      motion = motion.add(0, 0.2, 0);
    }

    mc.player.setDeltaMovement(motion);
    mc.player.fallDistance = 0f;
  }
}
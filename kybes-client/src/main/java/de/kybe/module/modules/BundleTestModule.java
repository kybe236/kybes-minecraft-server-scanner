package de.kybe.module.modules;

import de.kybe.module.ToggleableModule;
import de.kybe.settings.BooleanSetting;
import net.minecraft.network.protocol.game.ServerboundSelectBundleItemPacket;

import static de.kybe.Constants.mc;

public class BundleTestModule extends ToggleableModule {
    public BooleanSetting invalidPackets = (BooleanSetting) new BooleanSetting("Invalid Packets", false).onChange((aBoolean, aBoolean2) -> {
      if (aBoolean) {
        setToggled(false);
        if (mc.player == null || mc.getConnection() == null) {
          System.out.println("Player or connection is null, cannot send packets.");
          return;
        }

        ServerboundSelectBundleItemPacket packet = new ServerboundSelectBundleItemPacket(mc.player.getInventory().getSelectedSlot(), -1);
        mc.getConnection().send(packet);
        packet = new ServerboundSelectBundleItemPacket(mc.player.getInventory().getSelectedSlot(), 69);
        mc.getConnection().send(packet);
        packet = new ServerboundSelectBundleItemPacket(mc.player.getInventory().getSelectedSlot(), -69);
        mc.getConnection().send(packet);
      }
    });

    public BundleTestModule() {
        super("Bundle Test");

        addSetting(invalidPackets);
    }
}

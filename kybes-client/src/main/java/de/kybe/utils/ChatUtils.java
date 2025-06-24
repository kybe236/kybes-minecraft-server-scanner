package de.kybe.utils;

import net.minecraft.network.chat.Component;

import java.awt.*;

import static de.kybe.Constants.mc;

public class ChatUtils {
  public static void print(String message) {
    mc.gui.getChat().addMessage(Component.literal("[KYBE] ").withColor(Color.RED.getRGB()).append(Component.literal(message).withColor(Color.WHITE.getRGB())));
  }

  public static void print(Component message) {
    mc.gui.getChat().addMessage(Component.literal("[KYBE] ").withColor(Color.RED.getRGB()).append(message));
  }
}

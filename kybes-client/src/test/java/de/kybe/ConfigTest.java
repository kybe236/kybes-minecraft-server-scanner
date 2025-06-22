package de.kybe;

import de.kybe.config.Config;
import de.kybe.module.ModuleManager;
import de.kybe.module.ToggleableModule;
import de.kybe.settings.NumberSetting;
import de.kybe.settings.Setting;
import de.kybe.settings.StringSetting;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

public class ConfigTest {
  private static final Path TEST_PATH = Path.of("test_kybe.conf");

  @AfterEach
  public void cleanup() throws IOException {
    Files.deleteIfExists(TEST_PATH);
    ModuleManager.clearModules();
  }

  @Test
  public void testSaveAndLoadConfig() {
    TestToggleableModule testModule = new TestToggleableModule("TestModule");
    testModule.setToggled(true);

    NumberSetting<Integer> speed = new NumberSetting<>("Speed", 10);
    StringSetting mode = new StringSetting("Mode", "Fast");
    testModule.getSettings().add(speed);
    testModule.getSettings().add(mode);

    ModuleManager.register(testModule);

    Config.save(TEST_PATH);

    testModule.setToggled(false);
    speed.setValue(0);
    mode.setValue("Slow");

    Config.load(TEST_PATH);

    assertTrue(testModule.isToggled(), "Module toggled state should be true after load");
    assertEquals(10, speed.getValue(), "Speed setting should be 10 after load");
    assertEquals("Fast", mode.getValue(), "Mode setting should be 'Fast' after load");
  }

  private static class TestToggleableModule extends ToggleableModule {
    private final List<Setting<?>> settings = new ArrayList<>();

    public TestToggleableModule(String name) {
      super(name);
    }

    @Override
    public List<Setting<?>> getSettings() {
      return settings;
    }
  }
}

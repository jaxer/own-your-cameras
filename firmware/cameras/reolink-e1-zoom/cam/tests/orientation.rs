use cam::orientation::Orientation;
#[test]
fn stock_orientation_is_read_without_changing_axis_meaning() {
    let stock = "<isp_config_t><channel mirror=\"0\" flip=\"0\" /></isp_config_t>";
    assert_eq!(
        Orientation::from_vendor(stock).unwrap().sensor_words(),
        [0, 1, 0]
    );
    assert_eq!(
        Orientation::from_vendor(&stock.replace("mirror=\"0\"", "mirror=\"1\""))
            .unwrap()
            .sensor_words(),
        [0, 0, 0]
    );
    assert_eq!(
        Orientation::from_vendor(&stock.replace("flip=\"0\"", "flip=\"1\""))
            .unwrap()
            .sensor_words(),
        [0, 1, 1]
    );
    for bad in [
        "",
        "<channel mirror=\"0\" />",
        "<channel mirror=\"2\" flip=\"0\" />",
        "<channel mirror=\"0\" mirror=\"1\" flip=\"0\" />",
    ] {
        assert!(Orientation::from_vendor(bad).is_err());
    }
}

#[test]
fn sensor_setting_requires_matching_readback() {
    use cam::zoom::{Error, Isp, WORDS};
    struct Sensor {
        words: [u32; WORDS],
        wrong: bool,
    }
    impl Isp for Sensor {
        fn get(&mut self, item: u32, words: &mut [u32; WORDS]) -> Result<(), Error> {
            assert_eq!(item, 44);
            *words = self.words;
            if self.wrong {
                words[1] ^= 1;
            }
            Ok(())
        }
        fn set(&mut self, item: u32, words: &[u32; WORDS]) -> Result<(), Error> {
            assert_eq!(item, 44);
            self.words = *words;
            Ok(())
        }
        fn sleep(&mut self, _: std::time::Duration) {}
    }
    let orientation = Orientation::from_vendor("<channel mirror=\"1\" flip=\"0\" />").unwrap();
    let mut isp = Sensor {
        words: [0; WORDS],
        wrong: false,
    };
    orientation.apply(&mut isp).unwrap();
    assert_eq!(&isp.words[..3], &[0, 0, 0]);
    isp.wrong = true;
    assert!(orientation.apply(&mut isp).is_err());
}

extern crate ordered_float;

use self::ordered_float::NotNaN;

static NB_MARKERS: usize = 5;
static CONV_MIN: usize=500;

#[derive(Debug)]
pub struct P2 {
    count : usize,
    heights: Vec<NotNaN<f64>>,
    positions: Vec<usize>,
    npos: Vec<NotNaN<f64>>,
    incr: Vec<NotNaN<f64>>,
}

impl P2 {

    pub fn new (p: f64) -> Self {
        P2 {
            count: 0,
            heights: Vec::with_capacity(5),
            positions: (1..(NB_MARKERS+1)).collect::<Vec<usize>>(),
            npos: vec![1., 1.+2.*p, 1.+4.*p, 3.+2.*p, 5.].into_iter().map(|x| NotNaN::new(x).unwrap()).collect::<Vec<NotNaN<f64>>>(),
            incr: vec![0., p/2., p, (1.+p) / 2., 1.].into_iter().map(|x| NotNaN::new(x).unwrap()).collect::<Vec<NotNaN<f64>>>(),
        }
    }

    fn parabolic_formula (&self, i: usize, d: f64) -> NotNaN<f64> {
        assert!(i>=1 && i<=3);

        let qi = self.heights[i];
        let qim1 = self.heights[i-1];
        let qip1 = self.heights[i+1];
        let ni = NotNaN::new(self.positions[i] as f64).unwrap();
        let nip1 = NotNaN::new(self.positions[i+1] as f64).unwrap();
        let nim1 = NotNaN::new(self.positions[i-1] as f64).unwrap();
        let d = NotNaN::new(d).unwrap();

        let mut ret = (nip1 - ni - d) * (qi -qim1) / (ni - nim1);
        ret += (ni - nim1 +d) * (qip1 - qi) / (nip1 - ni);
        ret *= d / (nip1 - nim1);
        ret + qi
    }

    fn linear_formula(&self, i: usize, d: f64) -> NotNaN<f64> {
        assert!(i>=1 && i<=3);

        let num = if d > 0. { self.heights[i+1] - self.heights[i] } else { self.heights[i] - self.heights[i-1] };
        let den =  if d > 0. { self.positions[i+1] - self.positions[i] } else { self.positions[i] - self.positions[i-1] };
        let den = NotNaN::new(den as f64).unwrap();

        let d = NotNaN::new(d).unwrap();

        self.heights[i] + d * num / den
    }

    fn adjust(&mut self) {
        for i in 1..4 {
            let d = self.npos[i] - (self.positions[i] as f64);
            let d = d.into_inner();

            if ((d >= 1.) && ((self.positions[i+1]-self.positions[i]) > 1)) ||
                ((d<= -1.) && ((self.positions[i]-self.positions[i-1]) > 1)) {

                let d = if d >= 0. {1.} else {-1.};
                let new_height = self.parabolic_formula(i, d);

                if (self.heights[i-1] < new_height) && (new_height < self.heights[i+1]) {
                    self.heights[i] = new_height;
                }
                else {
                    self.heights[i] = self.linear_formula(i, d);
                }

                self.positions[i] = if d >= 0. {self.positions[i]+1} else { self.positions[i]-1 };
            }
        }
    }


    pub fn new_sample (&mut self, sample: f64) {
        self.count += 1;
        let sample = NotNaN::new(sample).unwrap();
        let hlen = self.heights.len();
        if hlen < 5 {
            self.heights.push(sample);
            if (hlen+1) == 5 {
                self.heights.sort();
            }
        }
        else {

            let mut k = 1;
            if sample < self.heights[0] {
                self.heights[0] = sample;
            }
            else {
                while (sample >= self.heights[k]) && (k<=3) {
                    k += 1;
                }
                if sample > self.heights[4] {
                    self.heights[4] = sample;
                    assert!(k == 4);
                }
            }

            for i in k..5 {
                self.positions[i] += 1;
            }

            for i in 0..5 {
                self.npos[i] = self.npos[i] + self.incr[i]
            }


            self.adjust ();
        }

    }

    pub fn get_quantile(&self) -> Option<f64> {
        if self.count > CONV_MIN {
            Some(self.heights[2].into_inner())
        }
        else {
            None
        }
    }

    pub fn print(&self) {
        println!("Heights: {:?}\nPos: {:?}\nnpos {:?}\ndn {:?}",
                 self.heights.iter().map(|x| x.into_inner()).collect::<Vec<f64>>(),
                 self.positions,
                 self.npos.iter().map(|x| x.into_inner()).collect::<Vec<f64>>(),
                 self.incr.iter().map(|x| x.into_inner()).collect::<Vec<f64>>(),
                 );
    }
}

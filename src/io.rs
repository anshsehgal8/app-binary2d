use std::collections::HashMap;
use hdf5::{File, Group, H5Type};
use io_logical::verified;
use io_logical::nicer_hdf5;
use io_logical::nicer_hdf5::{H5Read, H5Write};
use crate::scheme::{Hydrodynamics, Conserved, State, BlockData};
use crate::Tasks;




// ============================================================================
pub trait AssociatedH5Type { type H5Type: H5Type; }
pub trait SynonymForH5Type<T: H5Type + Clone>: AssociatedH5Type<H5Type=T> + Into<T> + From<T> { }
pub trait H5Conserved<T: H5Type + Clone>: Conserved + SynonymForH5Type<T> { }



impl AssociatedH5Type for hydro_iso2d::Conserved { type H5Type = [f64; 3]; }
impl SynonymForH5Type<[f64; 3]> for hydro_iso2d::Conserved {}
impl H5Conserved<[f64; 3]> for hydro_iso2d::Conserved {}

impl AssociatedH5Type for hydro_euler::euler_2d::Conserved { type H5Type = [f64; 4]; }
impl SynonymForH5Type<[f64; 4]> for hydro_euler::euler_2d::Conserved {}
impl H5Conserved<[f64; 4]> for hydro_euler::euler_2d::Conserved {}




// ============================================================================
impl nicer_hdf5::H5Read for Tasks
{
    fn read(group: &Group, name: &str) -> hdf5::Result<Self>
    {
        nicer_hdf5::read_as_keyed_vec(group, name)
    }    
}
impl nicer_hdf5::H5Write for Tasks
{
    fn write(&self, group: &Group, name: &str) -> hdf5::Result<()>
    {
        nicer_hdf5::write_as_keyed_vec(self.clone(), group, name)
    }
}




// ============================================================================
fn write_state<C: H5Conserved<T>, T: H5Type + Clone>(group: &Group, state: &State<C>, block_data: &Vec<BlockData<C>>) -> hdf5::Result<()>
{
    let state_group = group.create_group("state")?;
    let cons = state_group.create_group("conserved")?;

    for (b, u) in block_data.iter().zip(&state.conserved)
    {
        u.mapv(C::into).write(&cons, &format!("0:{:03}-{:03}", b.index.0, b.index.1))?
    }
    state.time.write(&state_group, "time")?;
    state.iteration.write(&state_group, "iteration")?;
    Ok(())
}

pub fn read_state<H: Hydrodynamics<Conserved=C>, C: H5Conserved<T>, T: H5Type + Clone>(_: &H) -> impl Fn(verified::File) -> hdf5::Result<State<C>>
{
    |file| {
        let file = File::open(file.to_string())?;
        let state_group = file.group("state")?;
        let cons = state_group.group("conserved")?;
        let mut conserved = Vec::new();

        for key in cons.member_names()?
        {
            let u = ndarray::Array2::<C::H5Type>::read(&cons, &key)?;
            conserved.push(u.mapv(C::from).to_shared());
        }
        let time      = f64::read(&state_group, "time")?;
        let iteration = num::rational::Ratio::<i64>::read(&state_group, "iteration")?;

        let result = State{
            conserved: conserved,
            time: time,
            iteration: iteration,
        };
        Ok(result)
    }
}

fn write_tasks(group: &Group, tasks: &Tasks) -> hdf5::Result<()>
{
    tasks.write(group, "tasks")
}

pub fn read_tasks(file: verified::File) -> hdf5::Result<Tasks>
{
    let file = File::open(file.to_string())?;
    Tasks::read(&file, "tasks")
}

fn write_model(group: &Group, model: &HashMap::<String, kind_config::Value>) -> hdf5::Result<()>
{
    kind_config::io::write_to_hdf5(&group.create_group("model")?, &model)
}

pub fn read_model(file: verified::File) -> hdf5::Result<HashMap::<String, kind_config::Value>>
{
    kind_config::io::read_from_hdf5(&File::open(file.to_string())?.group("model")?)
}

pub fn write_time_series<T: H5Type>(filename: &str, time_series: &Vec<T>) -> hdf5::Result<()>
{
    let file = File::create(filename)?;
    time_series.write(&file, "time_series")
}

pub fn read_time_series<T: H5Type>(file: verified::File) -> hdf5::Result<Vec<T>>
{
    let file = File::open(file.to_string())?;
    Vec::<T>::read(&file, "time_series")
}




// ============================================================================
pub fn write_checkpoint<C: H5Conserved<T>, T: H5Type + Clone>(
    filename: &str,
    state: &State<C>,
    block_data: &Vec<BlockData<C>>,
    model: &HashMap::<String, kind_config::Value>,
    tasks: &Tasks) -> hdf5::Result<()>
{
    let file = File::create(filename)?;

    write_state(&file, &state, block_data)?;
    write_tasks(&file, &tasks)?;
    write_model(&file, &model)?;

    Ok(())
}
